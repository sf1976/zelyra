'use strict';

const fs = require('node:fs');
const path = require('node:path');

const packages = [
  ['vscode-languageclient', 'License.txt'],
  ['vscode-jsonrpc', 'License.txt'],
  ['vscode-languageserver-protocol', 'License.txt'],
  ['vscode-languageserver-types', 'License.txt'],
  ['semver', 'LICENSE'],
  ['minimatch', 'LICENSE'],
  ['brace-expansion', 'LICENSE'],
  ['balanced-match', 'LICENSE.md']
];

const sections = packages.map(([name, license]) => {
  const directory = path.join(__dirname, 'node_modules', name);
  const manifest = JSON.parse(fs.readFileSync(path.join(directory, 'package.json'), 'utf8'));
  const body = fs.readFileSync(path.join(directory, license), 'utf8').trim();
  return `${name} ${manifest.version}\n${'='.repeat(name.length + manifest.version.length + 1)}\n${body}`;
});

fs.mkdirSync(path.join(__dirname, 'dist'), { recursive: true });
fs.writeFileSync(
  path.join(__dirname, 'dist', 'THIRD-PARTY-NOTICES.txt'),
  `${sections.join('\n\n')}\n`,
  'utf8'
);
