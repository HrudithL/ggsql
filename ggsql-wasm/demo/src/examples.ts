export interface Example {
  name: string;
  query: string;
  section: string;
}

export const examples: Example[] = [
  // === Layers ===
  {
    section: "Layers",
    name: "Area",
    query: `VISUALISE FROM ggsql:airquality
DRAW area 
  MAPPING Date AS x, Wind AS y`,
  },
  {
    section: "Layers",
    name: "Bar",
    query: `VISUALISE FROM ggsql:penguins
DRAW bar
    MAPPING species AS x`,
  },
  {
    section: "Layers",
    name: "Boxplot",
    query: `VISUALISE FROM ggsql:penguins
DRAW boxplot
  MAPPING species AS x, bill_len AS y, island AS fill`,
  },
  {
    section: "Layers",
    name: "Density",
    query: `VISUALISE bill_dep AS x, species AS colour FROM ggsql:penguins
  DRAW density MAPPING body_mass AS weight`,
  },
  {
    section: "Layers",
    name: "Histogram",
    query: `VISUALISE FROM ggsql:penguins
DRAW histogram
    MAPPING body_mass AS x`,
  },
  {
    section: "Layers",
    name: "Line",
    query: `VISUALISE FROM ggsql:airquality
DRAW line
    MAPPING Day AS x, Temp AS y, Month AS color`,
  },
  {
    section: "Layers",
    name: "Path",
    query: `WITH df(x, y, id) AS (VALUES
    (1.0, 1.0, 'A'),
    (2.0, 1.0, 'A'),
    (1.0, 3.0, 'A'),
    (3.0, 1.0, 'B'),
    (2.0, 3.0, 'B'),
    (3.0, 3.0, 'B')
)
VISUALIZE x, y FROM df
DRAW line
    MAPPING id AS colour`,
  },
  {
    section: "Layers",
    name: "Point",
    query: `SELECT * FROM ggsql:penguins
VISUALISE
DRAW point MAPPING bill_len AS x, bill_dep AS y, body_mass AS size, species AS color
LABEL title => 'Penguin Measurements', x => 'Bill Length (mm)', y => 'Bill Depth (mm)'`,
  },
  {
    section: "Layers",
    name: "Polygon",
    query: `WITH df(x, y, id) AS (VALUES
    (1.0, 1.0, 'A'),
    (2.0, 1.0, 'A'),
    (1.0, 3.0, 'A'),
    (3.0, 1.0, 'B'),
    (2.0, 3.0, 'B'),
    (3.0, 3.0, 'B')
)
VISUALIZE x, y FROM df
DRAW polygon
    MAPPING id AS colour`,
  },
  {
    section: "Layers",
    name: "Ribbon",
    query: `  VISUALISE FROM ggsql:airquality
  DRAW ribbon
    MAPPING Date AS x, Wind AS ymin, Temp AS ymax`,
  },
  {
    section: "Layers",
    name: "Violin",
    query: `VISUALISE species AS x, bill_dep AS y FROM ggsql:penguins
  DRAW violin`,
  },
  // === Scales ===
  {
    section: "Scales",
    name: "Binned",
    query: `VISUALISE bill_len AS x, bill_dep AS y, body_mass AS color FROM ggsql:penguins
DRAW point
SCALE BINNED color TO viridis`,
  },
  {
    section: "Scales",
    name: "Continuous",
    query: `VISUALISE bill_len AS x, bill_dep AS y FROM ggsql:penguins
DRAW point
SCALE x FROM [0, null]`,
  },
  {
    section: "Scales",
    name: "Discrete",
    query: `VISUALISE bill_len AS x, bill_dep AS y, island AS shape, island AS color FROM ggsql:penguins
DRAW point
  SETTING size => 6
SCALE shape TO ['star', 'circle', 'diamond']
SCALE color`,
  },
  {
    section: "Scales",
    name: "Identity",
    query: `WITH t(category, value, style) AS (VALUES
      ('A', 45, 'forestgreen'),
      ('B', 72, '#3401e3'),
      ('C', 38, 'hsl(150deg 30% 60%)')
)
VISUALISE category AS x, value AS y, style AS fill FROM t
DRAW bar
SCALE IDENTITY fill`,
  },
  {
    section: "Scales",
    name: "Ordinal",
    query: `VISUALISE Ozone AS x, Temp AS y FROM ggsql:airquality
DRAW point
    MAPPING Month AS color
SCALE ORDINAL color
    RENAMING * => '{}th month'`,
  },
  {
    section: "Scales",
    name: "Faceting",
    query: `VISUALISE sex AS x FROM ggsql:penguins
DRAW bar
FACET species
SCALE panel FROM ['Adelie', null]
    RENAMING null => 'The rest'`,
  },

  // === Aesthetics ===
  {
    section: "Aesthetics",
    name: "Position",
    query: `SELECT * FROM ggsql:penguins
VISUALISE
DRAW point MAPPING bill_len AS x, bill_dep AS y`,
  },
  {
    section: "Aesthetics",
    name: "Fill",
    query: `VISUALISE FROM ggsql:penguins
DRAW point
    MAPPING bill_dep AS x, body_mass AS y, species AS fill
    SETTING stroke => null
SCALE color TO category10`,
  },
  {
    section: "Aesthetics",
    name: "Opacity",
    query: `VISUALISE FROM ggsql:airquality
DRAW area 
  MAPPING Date AS x, Wind AS y
  SETTING opacity => 0.2`,
  },
  {
    section: "Aesthetics",
    name: "Linetype",
    query: `VISUALISE FROM ggsql:airquality
DRAW line
  MAPPING Day AS x, Temp AS y, Month AS linetype
SCALE ORDINAL linetype`,
  },
  {
    section: "Aesthetics",
    name: "Linewidth",
    query: `VISUALISE FROM ggsql:airquality
DRAW line
  MAPPING Day AS x, Temp AS y, Month AS colour
  SETTING linewidth => 5`,
  },
  {
    section: "Aesthetics",
    name: "Shape",
    query: `VISUALISE FROM ggsql:penguins
DRAW point
    MAPPING bill_dep AS x, body_mass AS y, species AS shape
    SETTING linewidth => 1, size => 5
SCALE shape TO ['star', 'bowtie', 'square-plus']`,
  },
  {
    section: "Aesthetics",
    name: "Size",
    query: `SELECT * FROM ggsql:penguins
VISUALISE
DRAW point MAPPING bill_len AS x, bill_dep AS y, body_mass AS size
LABEL title => 'Penguin Measurements', x => 'Bill Length (mm)', y => 'Bill Depth (mm)'`,
  },

  // === Tables (TABULATE) ===
  {
    section: "Tables",
    name: "Minimal",
    query: `-- The minimal TABULATE query: bare TABULATE renders every column,
-- in source order, with default gt-style formatting.
SELECT
  'Acme'             AS company, 120000 AS revenue, 18 AS employees UNION ALL SELECT
  'Globex',                       98000,            12              UNION ALL SELECT
  'Initech',                      54000,             7              UNION ALL SELECT
  'Umbrella',                    210000,            24              UNION ALL SELECT
  'Stark Industries',            480000,            55
TABULATE`,
  },
  {
    section: "Tables",
    name: "Title + thousands",
    query: `-- LABEL adds a styled header; FORMAT … RENAMING applies a printf-style
-- numeric format. The '\\'' flag is locale-aware thousands grouping.
SELECT
  'Acme'             AS company, 1200000 AS revenue, 1825 AS employees UNION ALL SELECT
  'Globex',                       980000,             1212              UNION ALL SELECT
  'Initech',                      540500,              704              UNION ALL SELECT
  'Umbrella',                   2100000,             2430              UNION ALL SELECT
  'Stark Industries',           4800500,             5512
TABULATE company, revenue, employees
FORMAT revenue, employees RENAMING * => '{:num %\\'d}'
LABEL
  title    => 'Top customers, FY26',
  subtitle => 'Revenue and headcount snapshot'`,
  },
  {
    section: "Tables",
    name: "Spanners",
    query: `-- FORMAT SPAN groups columns under a shared header row; spanners can
-- nest. STUB lifts a column out as a row identifier.
SELECT
  'Acme'             AS company,
  1200000            AS revenue_2025, 1825 AS employees_2025,
  1450000            AS revenue_2026, 1990 AS employees_2026 UNION ALL SELECT
  'Globex',           980000, 1212, 1100000, 1305            UNION ALL SELECT
  'Initech',          540500,  704,  610000,  742            UNION ALL SELECT
  'Umbrella',        2100000, 2430, 2380000, 2615
TABULATE company, revenue_2025, employees_2025, revenue_2026, employees_2026
FORMAT STUB company
FORMAT SPAN revenue_2025, employees_2025 AS y2025
FORMAT SPAN revenue_2026, employees_2026 AS y2026
FORMAT SPAN y2025, y2026                 AS snapshot
FORMAT revenue_2025, revenue_2026, employees_2025, employees_2026
  RENAMING * => '{:num %\\'d}'
LABEL
  revenue_2025   => 'Revenue (USD)',
  employees_2025 => 'Headcount',
  revenue_2026   => 'Revenue (USD)',
  employees_2026 => 'Headcount',
  y2025          => 'FY2025',
  y2026          => 'FY2026',
  snapshot       => 'Two-year snapshot'`,
  },
  {
    section: "Tables",
    name: "Widths + align",
    query: `-- FORMAT … SETTING width fixes column widths (table-layout: fixed);
-- align overrides the auto-aligned default.
SELECT
  'Acme'             AS company, 1200000 AS revenue, 18 AS employees UNION ALL SELECT
  'Globex',                       980000,            12              UNION ALL SELECT
  'Initech',                      540500,             7              UNION ALL SELECT
  'Umbrella',                    2100000,            24              UNION ALL SELECT
  'Stark Industries',            4800500,            55
TABULATE company, revenue, employees
FORMAT company   SETTING width => '200px'
FORMAT revenue   SETTING width => '120px', align => 'right'
  RENAMING * => '{:num %\\'d}'
FORMAT employees SETTING width => '100px', align => 'center'`,
  },
  {
    section: "Tables",
    name: "Highlight",
    query: `-- HIGHLIGHT <col> FILTER <SQL predicate> SETTING <key> => <value>
-- flags individual cells whose row matches the predicate.
SELECT * FROM (VALUES
  ('Alice',  92, 'A'),
  ('Bob',    58, 'F'),
  ('Carla',  74, 'C'),
  ('Dan',    45, 'F'),
  ('Eve',    88, 'B'),
  ('Frank',  67, 'D')
) AS students(name, score, grade)
TABULATE name, score, grade
HIGHLIGHT score
  FILTER score < 60
  SETTING face => 'bold', color => 'red'`,
  },
  {
    section: "Tables",
    name: "Facet groups",
    query: `-- FACET <col> groups body rows by the named column. A heading row
-- appears before each group; the grouping column drops out of the body.
SELECT * FROM (VALUES
  ('Hardware', 'Widget',     45000, 450),
  ('Hardware', 'Gadget',     62000, 620),
  ('Software', 'Gizmo',      38000, 380),
  ('Software', 'Doohickey',  51000, 510)
) AS sales(category, product, revenue, units)
TABULATE product, revenue, units
FACET category
FORMAT revenue RENAMING * => '{:num %\\'d}'`,
  },
];
