-- Fictional demo data for the disposable Zelyra machine-management starter.
-- Safe to import more than once: INSERT IGNORE never updates existing records.
-- No real people, companies, or operating sites are represented here.

INSERT IGNORE INTO departments (code, name, site, manager) VALUES
    ('D-CNC', 'Precision Workshop', 'Northstar Campus · Hall A', 'Mara Linden'),
    ('D-FAB', 'Fabrication', 'Northstar Campus · Hall B', 'Jonas Vale'),
    ('D-ASSEMBLY', 'Assembly Lines', 'Northstar Campus · Hall C', 'Leonie Hart'),
    ('D-QUALITY', 'Quality Lab', 'Northstar Campus · Lab 2', 'Noah Winter'),
    ('D-LOGISTICS', 'Internal Logistics', 'Northstar Campus · Yard 1', 'Emil Rowan'),
    ('D-ENERGY', 'Utilities & Energy', 'Northstar Campus · Plant Room', 'Tessa Brook');

INSERT IGNORE INTO machines (
    number, name, manufacturer, model, serial_number, category,
    department_id, commissioned_year, operating_hours, status, active
)
SELECT demo.number, demo.name, demo.manufacturer, demo.model, demo.serial_number,
       demo.category, departments.id, demo.commissioned_year,
       demo.operating_hours, demo.status, demo.active
FROM departments
JOIN (
    SELECT 'ZLY-DEMO-001' AS number, 'Five-axis milling center' AS name,
           'Asterion Works' AS manufacturer, 'AX-500' AS model,
           'AST-AX5-24001' AS serial_number, 'CNC' AS category,
           'D-CNC' AS department_code, 2021 AS commissioned_year,
           3840 AS operating_hours, 'operational' AS status, TRUE AS active
    UNION ALL SELECT 'ZLY-DEMO-002', 'Compact turning cell', 'Nordwerk Motion', 'NT-220', 'NWM-NT2-24002', 'CNC', 'D-CNC', 2020, 6120, 'operational', TRUE
    UNION ALL SELECT 'ZLY-DEMO-003', 'Vertical machining center', 'Asterion Works', 'AV-310', 'AST-AV3-24003', 'CNC', 'D-CNC', 2019, 9280, 'maintenance', TRUE
    UNION ALL SELECT 'ZLY-DEMO-004', 'Precision grinder', 'Ferrum Labs', 'FG-80', 'FER-FG8-24004', 'CNC', 'D-CNC', 2022, 1760, 'operational', TRUE
    UNION ALL SELECT 'ZLY-DEMO-005', 'Robotic tending unit', 'Kestrel Automation', 'KR-12', 'KES-KR1-24005', 'Robotics', 'D-CNC', 2023, 940, 'standby', TRUE
    UNION ALL SELECT 'ZLY-DEMO-006', 'Fiber laser cutter', 'Lumenforge', 'LF-4000', 'LUM-LF4-24006', 'Cutting', 'D-FAB', 2020, 7250, 'operational', TRUE
    UNION ALL SELECT 'ZLY-DEMO-007', 'Hydraulic press brake', 'Ferrum Labs', 'PB-160', 'FER-PB1-24007', 'Forming', 'D-FAB', 2018, 11400, 'maintenance', TRUE
    UNION ALL SELECT 'ZLY-DEMO-008', 'Cobot welding station', 'Kestrel Automation', 'KW-6', 'KES-KW6-24008', 'Welding', 'D-FAB', 2022, 2890, 'operational', TRUE
    UNION ALL SELECT 'ZLY-DEMO-009', 'Waterjet cutting table', 'Blueforge Systems', 'BW-3020', 'BLU-BW3-24009', 'Cutting', 'D-FAB', 2021, 4360, 'operational', TRUE
    UNION ALL SELECT 'ZLY-DEMO-010', 'Tube forming line', 'Nordwerk Motion', 'TF-90', 'NWM-TF9-24010', 'Forming', 'D-FAB', 2019, 8670, 'standby', TRUE
    UNION ALL SELECT 'ZLY-DEMO-011', 'Modular assembly cell A', 'Kestrel Automation', 'MA-240', 'KES-MA2-24011', 'Assembly', 'D-ASSEMBLY', 2022, 3170, 'operational', TRUE
    UNION ALL SELECT 'ZLY-DEMO-012', 'Modular assembly cell B', 'Kestrel Automation', 'MA-240', 'KES-MA2-24012', 'Assembly', 'D-ASSEMBLY', 2022, 3020, 'operational', TRUE
    UNION ALL SELECT 'ZLY-DEMO-013', 'Servo press station', 'Asterion Works', 'SP-45', 'AST-SP4-24013', 'Assembly', 'D-ASSEMBLY', 2020, 6970, 'maintenance', TRUE
    UNION ALL SELECT 'ZLY-DEMO-014', 'Automated screwdriving unit', 'Nordwerk Motion', 'AS-8', 'NWM-AS8-24014', 'Assembly', 'D-ASSEMBLY', 2023, 1180, 'operational', TRUE
    UNION ALL SELECT 'ZLY-DEMO-015', 'Conveyor inspection gate', 'Blueforge Systems', 'IG-12', 'BLU-IG1-24015', 'Inspection', 'D-ASSEMBLY', 2021, 4920, 'standby', TRUE
    UNION ALL SELECT 'ZLY-DEMO-016', 'Coordinate measuring machine', 'Meridian Metrology', 'MC-900', 'MER-MC9-24016', 'Metrology', 'D-QUALITY', 2021, 2580, 'operational', TRUE
    UNION ALL SELECT 'ZLY-DEMO-017', 'Optical surface scanner', 'Meridian Metrology', 'OS-4K', 'MER-OS4-24017', 'Metrology', 'D-QUALITY', 2023, 880, 'operational', TRUE
    UNION ALL SELECT 'ZLY-DEMO-018', 'Universal test frame', 'Ferrum Labs', 'UT-100', 'FER-UT1-24018', 'Testing', 'D-QUALITY', 2019, 5840, 'maintenance', TRUE
    UNION ALL SELECT 'ZLY-DEMO-019', 'Thermal cycling chamber', 'Lumenforge', 'TC-70', 'LUM-TC7-24019', 'Testing', 'D-QUALITY', 2020, 4460, 'operational', TRUE
    UNION ALL SELECT 'ZLY-DEMO-020', 'Digital height gauge', 'Meridian Metrology', 'DH-600', 'MER-DH6-24020', 'Metrology', 'D-QUALITY', 2024, 210, 'standby', TRUE
    UNION ALL SELECT 'ZLY-DEMO-021', 'Autonomous tugger fleet dock', 'Blueforge Systems', 'AT-3', 'BLU-AT3-24021', 'Logistics', 'D-LOGISTICS', 2022, 3630, 'operational', TRUE
    UNION ALL SELECT 'ZLY-DEMO-022', 'Pallet shuttle system', 'Nordwerk Motion', 'PS-12', 'NWM-PS1-24022', 'Logistics', 'D-LOGISTICS', 2021, 5380, 'operational', TRUE
    UNION ALL SELECT 'ZLY-DEMO-023', 'Smart vertical lift', 'Asterion Works', 'VL-18', 'AST-VL1-24023', 'Logistics', 'D-LOGISTICS', 2020, 7820, 'maintenance', TRUE
    UNION ALL SELECT 'ZLY-DEMO-024', 'Automated packing line', 'Kestrel Automation', 'AP-360', 'KES-AP3-24024', 'Logistics', 'D-LOGISTICS', 2023, 1560, 'operational', TRUE
    UNION ALL SELECT 'ZLY-DEMO-025', 'Dock leveler controller', 'Blueforge Systems', 'DL-5', 'BLU-DL5-24025', 'Logistics', 'D-LOGISTICS', 2019, 6410, 'standby', TRUE
    UNION ALL SELECT 'ZLY-DEMO-026', 'Main air compressor', 'Northstar Utilities', 'AC-75', 'NOR-AC7-24026', 'Utilities', 'D-ENERGY', 2020, 8240, 'operational', TRUE
    UNION ALL SELECT 'ZLY-DEMO-027', 'Heat recovery module', 'Lumenforge', 'HR-40', 'LUM-HR4-24027', 'Utilities', 'D-ENERGY', 2022, 2910, 'operational', TRUE
    UNION ALL SELECT 'ZLY-DEMO-028', 'Water treatment skid', 'Northstar Utilities', 'WT-18', 'NOR-WT1-24028', 'Utilities', 'D-ENERGY', 2019, 9180, 'maintenance', TRUE
    UNION ALL SELECT 'ZLY-DEMO-029', 'Roof solar inverter bank', 'Blueforge Systems', 'SI-250', 'BLU-SI2-24029', 'Energy', 'D-ENERGY', 2023, 1720, 'operational', TRUE
    UNION ALL SELECT 'ZLY-DEMO-030', 'Backup power generator', 'Northstar Utilities', 'PG-500', 'NOR-PG5-24030', 'Utilities', 'D-ENERGY', 2018, 1330, 'standby', TRUE
) AS demo ON departments.code = demo.department_code;
