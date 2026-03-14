#!/usr/bin/env python3
"""Fetch all RER/Transilien lines in Île-de-France and create KML files with routes and stops."""

import os
import time
import requests
import xml.etree.ElementTree as ET

OUTPUT_DIR = "idf/rer"
TRACES_API = "https://data.iledefrance-mobilites.fr/api/explore/v2.1/catalog/datasets/traces-des-lignes-de-transport-en-commun-idfm/records"
STOPS_API = "https://data.iledefrance-mobilites.fr/api/explore/v2.1/catalog/datasets/arrets-lignes/records"
PAGE_SIZE = 100


def fetch_all_pages(base_url, where_clause):
    """Fetch all records from paginated API."""
    records = []
    offset = 0
    session = requests.Session()
    session.verify = False
    import urllib3
    urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)
    while True:
        url = f"{base_url}?limit={PAGE_SIZE}&offset={offset}&where={where_clause}"
        try:
            resp = session.get(url, timeout=30)
            resp.raise_for_status()
            data = resp.json()
        except Exception as e:
            print(f"  Error at offset {offset}: {e}")
            break

        results = data.get("results", [])
        if not results:
            break
        records.extend(results)
        total = data.get("total_count", 0)
        offset += PAGE_SIZE
        if total and offset % 500 == 0:
            print(f"  ... {offset}/{total}")
        if offset >= total:
            break
        time.sleep(0.1)

    return records


def sanitize_filename(name):
    """Make a safe filename from line name."""
    replacements = {
        " ": "-", "/": "-", "'": "_", "(": "", ")": "",
        "é": "e", "è": "e", "ê": "e", "ë": "e",
        "à": "a", "â": "a", "ô": "o", "î": "i", "ï": "i",
        "ù": "u", "û": "u", "ü": "u", "ç": "c",
        "É": "E", "È": "E", "Ê": "E", "À": "A",
        "Ô": "O", "Î": "I", "Ç": "C",
        "œ": "oe", "Œ": "OE", "æ": "ae", "Æ": "AE",
    }
    for old, new in replacements.items():
        name = name.replace(old, new)
    return "".join(c for c in name if c.isalnum() or c in "-_.")


def hex_to_kml_bgr(hex_color):
    """Convert hex color (e.g. '9B9842') to KML BGR format."""
    hex_color = hex_color.lstrip("#")
    if len(hex_color) != 6:
        return "FF0000"
    r = hex_color[0:2]
    g = hex_color[2:4]
    b = hex_color[4:6]
    return b + g + r


def classify_line(short_name, long_name, operator):
    """Classify a Rail line as RER or Transilien."""
    name_upper = (short_name or "").upper()
    if name_upper in ("A", "B", "C", "D", "E"):
        return "rer", f"RER-{name_upper}"
    if name_upper == "TER":
        # Use long_name to disambiguate TER lines
        region = (long_name or "").replace("TER ", "").strip()
        safe = sanitize_filename(region) if region else "unknown"
        return "ter", f"TER-{safe}"
    return "transilien", f"transilien-{short_name}"


def make_kml(line_label, line_name, long_name, operator, color, shapes, stops):
    """Create a KML file for a rail line."""
    kml = ET.Element("kml", xmlns="http://www.opengis.net/kml/2.2")
    doc = ET.SubElement(kml, "Document")
    ET.SubElement(doc, "name").text = f"{line_label} - {long_name}"
    if operator:
        ET.SubElement(doc, "description").text = operator

    # Line style
    kml_color = "ff" + hex_to_kml_bgr(color) if color else "ffFF0000"
    style = ET.SubElement(doc, "Style", id="line_style")
    line_style = ET.SubElement(style, "LineStyle")
    ET.SubElement(line_style, "color").text = kml_color
    ET.SubElement(line_style, "width").text = "4"

    # Stop style with rail icon
    stop_style = ET.SubElement(doc, "Style", id="stop_style")
    icon_style = ET.SubElement(stop_style, "IconStyle")
    ET.SubElement(icon_style, "color").text = kml_color
    ET.SubElement(icon_style, "scale").text = "0.6"
    icon = ET.SubElement(icon_style, "Icon")
    ET.SubElement(icon, "href").text = "https://maps.google.com/mapfiles/kml/shapes/rail.png"

    # Route traces
    trace_folder = ET.SubElement(doc, "Folder")
    ET.SubElement(trace_folder, "name").text = "Tracé"

    for shape in shapes:
        geom = shape.get("geometry", {})
        geom_type = geom.get("type", "")
        coords_list = geom.get("coordinates", [])

        if geom_type == "MultiLineString":
            for segment in coords_list:
                pm = ET.SubElement(trace_folder, "Placemark")
                ET.SubElement(pm, "name").text = line_name
                ET.SubElement(pm, "styleUrl").text = "#line_style"
                ls = ET.SubElement(pm, "LineString")
                ET.SubElement(ls, "tessellate").text = "1"
                coord_str = " ".join(f"{lon},{lat},0" for lon, lat in segment)
                ET.SubElement(ls, "coordinates").text = coord_str
        elif geom_type == "LineString":
            pm = ET.SubElement(trace_folder, "Placemark")
            ET.SubElement(pm, "name").text = line_name
            ET.SubElement(pm, "styleUrl").text = "#line_style"
            ls = ET.SubElement(pm, "LineString")
            ET.SubElement(ls, "tessellate").text = "1"
            coord_str = " ".join(f"{lon},{lat},0" for lon, lat in coords_list)
            ET.SubElement(ls, "coordinates").text = coord_str

    # Stops
    if stops:
        stops_folder = ET.SubElement(doc, "Folder")
        ET.SubElement(stops_folder, "name").text = "Gares"
        seen = set()
        for stop in stops:
            name = stop.get("stop_name", "")
            lat = stop.get("stop_lat")
            lon = stop.get("stop_lon")
            if not lat or not lon or not name:
                continue
            if name in seen:
                continue
            seen.add(name)

            pm = ET.SubElement(stops_folder, "Placemark")
            ET.SubElement(pm, "name").text = name
            ET.SubElement(pm, "styleUrl").text = "#stop_style"
            point = ET.SubElement(pm, "Point")
            ET.SubElement(point, "coordinates").text = f"{lon},{lat},0"

    tree = ET.ElementTree(kml)
    ET.indent(tree, space="  ")
    return tree


def main():
    os.makedirs(OUTPUT_DIR, exist_ok=True)

    # Step 1: Fetch all Rail line traces
    print("Fetching Rail line traces (RER + Transilien)...")
    traces = fetch_all_pages(TRACES_API, "route_type%3D%22Rail%22")
    print(f"  Got {len(traces)} rail line traces")

    if not traces:
        print("  No traces found!")
        return

    # Group traces by route_id
    traces_by_id = {}
    line_info = {}
    for t in traces:
        rid = t.get("route_id", "")
        if not rid:
            continue
        if rid not in traces_by_id:
            traces_by_id[rid] = []
        shape = t.get("shape")
        if shape and shape.get("geometry"):
            traces_by_id[rid].append(shape)
        if rid not in line_info:
            line_info[rid] = {
                "short_name": t.get("route_short_name", ""),
                "long_name": t.get("route_long_name", ""),
                "color": t.get("route_color", ""),
                "operator": t.get("operatorname", ""),
            }

    print(f"  {len(traces_by_id)} unique rail lines")
    for rid in sorted(traces_by_id.keys()):
        info = line_info[rid]
        print(f"    {info['short_name']:>5} | {info['long_name'][:50]:50} | color={info['color']} | {info['operator']}")

    # Step 2: Fetch stops in batches
    print("Fetching rail stops...")
    stops_by_id = {}
    session = requests.Session()
    session.verify = False
    import urllib3
    urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)
    route_ids = sorted(traces_by_id.keys())
    batch_size = 10
    for batch_start in range(0, len(route_ids), batch_size):
        batch = route_ids[batch_start:batch_start + batch_size]
        if batch_start % 50 == 0:
            print(f"  ... stops batch {batch_start}/{len(route_ids)}")
        or_clauses = " OR ".join(f'id="{rid}"' for rid in batch)
        where = requests.utils.quote(or_clauses)
        offset = 0
        while True:
            url = f"{STOPS_API}?limit={PAGE_SIZE}&offset={offset}&where={where}"
            try:
                resp = session.get(url, timeout=15)
                resp.raise_for_status()
                data = resp.json()
                results = data.get("results", [])
                if not results:
                    break
                for s in results:
                    rid = s.get("id", "")
                    if rid:
                        stops_by_id.setdefault(rid, []).append(s)
                total = data.get("total_count", 0)
                offset += PAGE_SIZE
                if offset >= total:
                    break
            except Exception:
                break
        time.sleep(0.05)
    total_stops = sum(len(v) for v in stops_by_id.values())
    print(f"  Got stops for {len(stops_by_id)} lines ({total_stops} total stop-line associations)")

    # Step 3: Generate KML files
    print(f"Generating KML files in {OUTPUT_DIR}/...")
    count = 0
    for rid, shapes in sorted(traces_by_id.items()):
        info = line_info.get(rid, {})
        short_name = info.get("short_name", "")
        long_name = info.get("long_name", "")
        color = info.get("color", "")
        operator = info.get("operator", "")

        if not short_name:
            short_name = rid.replace("IDFM:", "")

        line_type, file_label = classify_line(short_name, long_name, operator)
        line_stops = stops_by_id.get(rid, [])

        tree = make_kml(file_label, short_name, long_name, operator, color, shapes, line_stops)

        filename = f"{sanitize_filename(file_label)}.kml"
        filepath = os.path.join(OUTPUT_DIR, filename)
        tree.write(filepath, xml_declaration=True, encoding="UTF-8")
        print(f"  {filename} ({len(shapes)} shapes, {len(set(s.get('stop_name','') for s in line_stops))} stops)")
        count += 1

    print(f"\nDone: {count} KML files written to {OUTPUT_DIR}/")


if __name__ == "__main__":
    main()
