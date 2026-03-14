import { test, expect } from '@playwright/test'

const rerAKml = `<?xml version="1.0" encoding="UTF-8"?>
<kml xmlns="http://www.opengis.net/kml/2.2">
<Document>
  <name>RER A</name>
  <Style id="line_style">
    <LineStyle><color>ff3221EB</color><width>4</width></LineStyle>
  </Style>
  <Style id="stop_style">
    <IconStyle>
      <color>ff3221EB</color>
      <scale>0.6</scale>
      <Icon><href>https://maps.google.com/mapfiles/kml/shapes/rail.png</href></Icon>
    </IconStyle>
  </Style>
  <Folder>
    <name>Tracé</name>
    <Placemark>
      <name>A</name>
      <styleUrl>#line_style</styleUrl>
      <LineString><coordinates>2.19,48.92,0 2.25,48.90,0 2.30,48.88,0</coordinates></LineString>
    </Placemark>
  </Folder>
  <Folder>
    <name>Gares</name>
    <Placemark>
      <name>Nanterre</name>
      <styleUrl>#stop_style</styleUrl>
      <Point><coordinates>2.19,48.92,0</coordinates></Point>
    </Placemark>
  </Folder>
</Document>
</kml>`

const rerEKml = `<?xml version="1.0" encoding="UTF-8"?>
<kml xmlns="http://www.opengis.net/kml/2.2">
<Document>
  <name>RER E</name>
  <Style id="line_style">
    <LineStyle><color>ff9A4EB9</color><width>4</width></LineStyle>
  </Style>
  <Style id="stop_style">
    <IconStyle>
      <color>ff9A4EB9</color>
      <scale>0.6</scale>
      <Icon><href>https://maps.google.com/mapfiles/kml/shapes/rail.png</href></Icon>
    </IconStyle>
  </Style>
  <Folder>
    <name>Tracé</name>
    <Placemark>
      <name>E</name>
      <styleUrl>#line_style</styleUrl>
      <LineString><coordinates>2.35,48.87,0 2.40,48.86,0 2.45,48.85,0</coordinates></LineString>
    </Placemark>
  </Folder>
  <Folder>
    <name>Gares</name>
    <Placemark>
      <name>Rosa Parks</name>
      <styleUrl>#stop_style</styleUrl>
      <Point><coordinates>2.37,48.87,0</coordinates></Point>
    </Placemark>
  </Folder>
</Document>
</kml>`

test.describe('MapPage - multiple KML layers', () => {
  test.beforeEach(async ({ page }) => {
    // Mock API routes for RER-A and RER-E
    await page.route('**/api/idf/rer/RER-A.kml', route =>
      route.fulfill({
        status: 200,
        contentType: 'application/vnd.google-earth.kml+xml',
        body: rerAKml,
      })
    )
    await page.route('**/api/idf/rer/RER-E.kml', route =>
      route.fulfill({
        status: 200,
        contentType: 'application/vnd.google-earth.kml+xml',
        body: rerEKml,
      })
    )
  })

  test('loading a second KML preserves the first layer colors', async ({ page }) => {
    // Load RER-A
    await page.goto('/map')
    const input = page.locator('.map-bar input')
    await input.fill('idf/rer/RER-A.kml')
    await page.locator('.load-btn').click()

    // Wait for the red line to appear (RER A color: #EB2132 -> kml ff3221EB)
    const rerAPath = page.locator('path[stroke="#EB2132"]').first()
    await expect(rerAPath).toBeVisible()

    // Load RER-E
    await input.fill('idf/rer/RER-E.kml')
    await page.locator('.load-btn').click()

    // Wait for the purple line to appear (RER E color: #B94E9A -> kml ff9A4EB9)
    const rerEPath = page.locator('path[stroke="#B94E9A"]').first()
    await expect(rerEPath).toBeVisible()

    // RER-A line should STILL be red
    await expect(rerAPath).toBeVisible()
    // RER-A line should NOT have taken RER-E's color
    expect(await rerAPath.getAttribute('stroke')).toBe('#EB2132')
  })
})
