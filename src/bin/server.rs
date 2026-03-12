use axum::{Router, extract::{Path, Query}, response::{IntoResponse, Html}, http::StatusCode};
use tower_http::cors::CorsLayer;
use serde::Deserialize;

const S3_BUCKET: &str = "kml-laurent";
const S3_REGION: &str = "eu-west-3";

async fn proxy_s3(Path(path): Path<String>) -> impl IntoResponse {
    let url = format!(
        "https://{}.s3.{}.amazonaws.com/{}",
        S3_BUCKET, S3_REGION, path
    );
    match reqwest::get(&url).await {
        Ok(resp) if resp.status().is_success() => {
            let body = resp.bytes().await.unwrap_or_default();
            (StatusCode::OK, [
                ("content-type", "application/vnd.google-earth.kml+xml"),
                ("access-control-allow-origin", "*"),
            ], body).into_response()
        }
        Ok(resp) => (StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::NOT_FOUND), "Not found").into_response(),
        Err(e) => (StatusCode::BAD_GATEWAY, format!("S3 fetch error: {}", e)).into_response(),
    }
}

#[derive(Deserialize)]
struct MapQuery {
    kml: Option<String>,
}

async fn map_viewer(Query(params): Query<MapQuery>) -> Html<String> {
    let kml_path = params.kml.unwrap_or_default();
    Html(format!(r#"<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <title>KML Viewer</title>
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <link rel="stylesheet" href="https://unpkg.com/leaflet@1.9.4/dist/leaflet.css" />
  <style>
    body {{ margin: 0; font-family: sans-serif; }}
    #map {{ position: absolute; top: 40px; bottom: 0; width: 100%; }}
    #bar {{ height: 40px; background: #333; color: #fff; display: flex; align-items: center; padding: 0 12px; gap: 8px; }}
    #bar input {{ flex: 1; padding: 6px; border-radius: 4px; border: none; font-size: 14px; }}
    #bar button {{ padding: 6px 14px; border-radius: 4px; border: none; background: #4CAF50; color: #fff; cursor: pointer; font-size: 14px; }}
    #bar button:hover {{ background: #45a049; }}
    .legend {{ background: white; padding: 8px 12px; border-radius: 6px; box-shadow: 0 1px 5px rgba(0,0,0,0.3); font-size: 13px; line-height: 1.6; max-height: 300px; overflow-y: auto; }}
    .legend-item {{ display: flex; align-items: center; gap: 6px; }}
    .legend-color {{ width: 20px; height: 3px; border-radius: 1px; }}
  </style>
</head>
<body>
  <div id="bar">
    <input id="kml-input" type="text" placeholder="KML path (e.g. bus/bus-1.kml)" value="{kml_path}" />
    <button onclick="loadKml()">Load</button>
  </div>
  <div id="map"></div>
  <script src="https://unpkg.com/leaflet@1.9.4/dist/leaflet.js"></script>
  <script src="https://unpkg.com/leaflet-omnivore@0.3.4/leaflet-omnivore.min.js"></script>
  <script>
    var map = L.map('map').setView([48.92, 2.19], 13);
    L.tileLayer('https://{{s}}.tile.openstreetmap.org/{{z}}/{{x}}/{{y}}.png', {{
      attribution: '&copy; OpenStreetMap contributors'
    }}).addTo(map);

    var currentLayer = null;
    var legend = null;

    function kmlColorToHex(kmlColor) {{
      if (!kmlColor || kmlColor.length !== 8) return '#3388ff';
      // KML color is aabbggrr
      var r = kmlColor.substring(6, 8);
      var g = kmlColor.substring(4, 6);
      var b = kmlColor.substring(2, 4);
      return '#' + r + g + b;
    }}

    function loadKml() {{
      var path = document.getElementById('kml-input').value.trim();
      if (!path) return;
      if (currentLayer) {{ map.removeLayer(currentLayer); currentLayer = null; }}
      if (legend) {{ map.removeControl(legend); legend = null; }}

      // Update URL
      history.replaceState(null, '', '/?kml=' + encodeURIComponent(path));

      // Fetch KML and parse for styles
      fetch('/' + path).then(r => r.text()).then(function(kmlText) {{
        var parser = new DOMParser();
        var doc = parser.parseFromString(kmlText, 'text/xml');

        // Extract styles
        var styles = {{}};
        doc.querySelectorAll('Style').forEach(function(s) {{
          var id = s.getAttribute('id');
          var lineEl = s.querySelector('LineStyle > color');
          if (id && lineEl) {{
            styles[id] = kmlColorToHex(lineEl.textContent.trim());
          }}
        }});

        // Extract folders for legend
        var folders = [];
        doc.querySelectorAll('Document > Folder').forEach(function(f) {{
          var name = f.querySelector('name');
          if (name) folders.push(name.textContent);
        }});

        // Parse with omnivore
        var layer = omnivore.kml.parse(kmlText);
        layer.eachLayer(function(l) {{
          // Try to apply KML style
          if (l.feature && l.feature.properties && l.feature.properties.styleUrl) {{
            var styleId = l.feature.properties.styleUrl.replace('#', '');
            if (styles[styleId]) {{
              if (l.setStyle) l.setStyle({{ color: styles[styleId], weight: 3 }});
            }}
          }}
          // Bind popup with name
          if (l.feature && l.feature.properties && l.feature.properties.name) {{
            l.bindPopup('<b>' + l.feature.properties.name + '</b>');
          }}
        }});
        layer.addTo(map);
        map.fitBounds(layer.getBounds().pad(0.1));
        currentLayer = layer;

        // Add legend for document name
        var docName = doc.querySelector('Document > name');
        if (docName) {{
          legend = L.control({{ position: 'bottomright' }});
          legend.onAdd = function() {{
            var div = L.DomUtil.create('div', 'legend');
            var html = '<b>' + docName.textContent + '</b><br>';
            Object.keys(styles).forEach(function(id) {{
              html += '<div class="legend-item"><span class="legend-color" style="background:' + styles[id] + '"></span>' + id.replace(/_/g, ' ') + '</div>';
            }});
            div.innerHTML = html;
            return div;
          }};
          legend.addTo(map);
        }}
      }});
    }}

    // Auto-load if kml param is set
    if (document.getElementById('kml-input').value) {{
      loadKml();
    }}
  </script>
</body>
</html>"#))
}

#[tokio::main]
async fn main() {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());

    let cors = CorsLayer::permissive();
    let app = Router::new()
        .route("/", axum::routing::get(map_viewer))
        .route("/{*path}", axum::routing::get(proxy_s3))
        .layer(cors);

    let addr = format!("0.0.0.0:{}", port);
    println!("Proxying S3 bucket {} on {}", S3_BUCKET, addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
