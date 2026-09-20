#!/usr/bin/env python3
"""
Builds an official standard VS Code .vsix package for the Zelyra extension.
Open Packaging Convention (OPC) compliant ZIP archive.
"""

import os
import sys
import json
import zipfile
from pathlib import Path

def build_vsix():
    repo_root = Path(__file__).resolve().parent.parent
    editors_dir = repo_root / "editors"
    output_vsix = editors_dir / "zelyra.vsix"
    
    pkg_file = editors_dir / "package.json"
    if not pkg_file.exists():
        print(f"Error: {pkg_file} not found!")
        sys.exit(1)
        
    with open(pkg_file, "r", encoding="utf-8") as f:
        pkg_data = json.load(f)
        
    version = pkg_data.get("version", "0.1.50")
    publisher = pkg_data.get("publisher", "sf1976")
    ext_id = pkg_data.get("name", "zelyra-language")
    display_name = pkg_data.get("displayName", "Zelyra")
    description = pkg_data.get("description", "Official syntax highlighting and language configuration for Zelyra (.zyl)")
    
    content_types_xml = '''<?xml version="1.0" encoding="utf-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="vsixmanifest" ContentType="text/xml"/>
  <Default Extension="json" ContentType="application/json"/>
  <Default Extension="png" ContentType="image/png"/>
  <Default Extension="md" ContentType="text/markdown"/>
  <Default Extension="txt" ContentType="text/plain"/>
</Types>'''

    vsixmanifest_xml = f'''<?xml version="1.0" encoding="utf-8"?>
<PackageManifest Version="2.0.0" xmlns="http://schemas.microsoft.com/developer/vsx-schema/2011" xmlns:d="http://schemas.microsoft.com/developer/vsx-schema-design/2011">
  <Metadata>
    <Identity Id="{ext_id}" Version="{version}" Language="en-US" Publisher="{publisher}"/>
    <DisplayName>{display_name}</DisplayName>
    <Description xml:space="preserve">{description}</Description>
    <Tags>zelyra,zyl,syntax,language</Tags>
    <Categories>Programming Languages</Categories>
    <GalleryFlags>Public</GalleryFlags>
    <Properties>
      <Property Id="Microsoft.VisualStudio.Code.Engine" Value="^1.60.0"/>
      <Property Id="Microsoft.VisualStudio.Code.ExtensionDependencies" Value=""/>
      <Property Id="Microsoft.VisualStudio.Code.ExtensionPack" Value=""/>
      <Property Id="Microsoft.VisualStudio.Code.LocalizedLanguages" Value=""/>
    </Properties>
    <License>extension/LICENSE</License>
    <Icon>extension/icon.png</Icon>
  </Metadata>
  <Installation>
    <InstallationTarget Id="Microsoft.VisualStudio.Code"/>
  </Installation>
  <Dependencies/>
  <Assets>
    <Asset Type="Microsoft.VisualStudio.Code.Manifest" Path="extension/package.json" Addressable="true"/>
    <Asset Type="Microsoft.VisualStudio.Services.Content.Details" Path="extension/README.md" Addressable="true"/>
    <Asset Type="Microsoft.VisualStudio.Services.Content.License" Path="extension/LICENSE" Addressable="true"/>
    <Asset Type="Microsoft.VisualStudio.Services.Icons.Default" Path="extension/icon.png" Addressable="true"/>
  </Assets>
</PackageManifest>'''

    print(f"Building Zelyra VS Code VSIX extension v{version}...")

    # Files to include inside extension/ folder
    files_to_pack = [
        ("package.json", "extension/package.json"),
        ("language-configuration.json", "extension/language-configuration.json"),
        ("syntaxes/zelyra.tmLanguage.json", "extension/syntaxes/zelyra.tmLanguage.json"),
        ("README.md", "extension/README.md"),
        ("LICENSE", "extension/LICENSE"),
        ("icon.png", "extension/icon.png"),
    ]

    with zipfile.ZipFile(output_vsix, "w", compression=zipfile.ZIP_DEFLATED) as zf:
        # 1. Open Packaging Convention metadata
        zf.writestr("[Content_Types].xml", content_types_xml)
        zf.writestr("extension.vsixmanifest", vsixmanifest_xml)
        
        # 2. Extension files
        for src_rel, arc_name in files_to_pack:
            src_path = editors_dir / src_rel
            if src_path.exists():
                zf.write(src_path, arc_name)
                print(f" + Added {arc_name} ({src_path.stat().st_size} bytes)")
            else:
                print(f" ! Warning: {src_path} not found, skipping")

    print(f"✓ VSIX built successfully: {output_vsix} ({output_vsix.stat().st_size} bytes)")

    # Copy to web public downloads directory if present
    target_dirs = [
        Path("/opt/siedelmann-laravel/app/public/downloads"),
        Path("/opt/siedelmann-laravel/app/public/editors"),
    ]
    for td in target_dirs:
        if td.exists():
            dest = td / "zelyra.vsix"
            dest.write_bytes(output_vsix.read_bytes())
            os.chmod(dest, 0o775)
            print(f"✓ Copied to web asset: {dest}")

if __name__ == "__main__":
    build_vsix()
