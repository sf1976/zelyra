'use strict';

const path = require('node:path');
const vscode = require('vscode');
const { LanguageClient } = require('vscode-languageclient/node');

let client;

async function activate(context) {
  const config = vscode.workspace.getConfiguration('zelyra');
  const executable = config.get('languageServer.path', 'zelyra').trim() || 'zelyra';
  const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
  const document = vscode.window.activeTextEditor?.document;
  const cwd = workspaceFolder?.uri.fsPath ?? (document ? path.dirname(document.uri.fsPath) : undefined);
  const outputChannel = vscode.window.createOutputChannel('Zelyra Language Server');
  const serverOptions = {
    command: executable,
    args: ['lsp'],
    options: { cwd }
  };
  const clientOptions = {
    documentSelector: [{ scheme: 'file', language: 'zelyra' }],
    outputChannel,
    traceOutputChannel: outputChannel
  };

  client = new LanguageClient('zelyra', 'Zelyra Language Server', serverOptions, clientOptions);
  context.subscriptions.push(outputChannel, client);
  await client.start();
}

async function deactivate() {
  if (client) {
    await client.stop();
    client = undefined;
  }
}

module.exports = { activate, deactivate };
