'use strict';

const assert = require('node:assert/strict');
const Module = require('node:module');
const path = require('node:path');
const test = require('node:test');

test('activates zelyra lsp from the workspace using the configured executable', async () => {
  const originalLoad = Module._load;
  const calls = {};
  const subscriptions = [];
  const fakeClient = {
    async start() { calls.started = true; },
    async stop() { calls.stopped = true; }
  };
  const fakeVscode = {
    workspace: {
      getConfiguration(section) {
        assert.equal(section, 'zelyra');
        return { get(key, fallback) {
          assert.equal(key, 'languageServer.path');
          assert.equal(fallback, 'zelyra');
          return '/custom/bin/zelyra';
        } };
      },
      workspaceFolders: [{ uri: { fsPath: '/workspace/project' } }]
    },
    window: {
      activeTextEditor: undefined,
      createOutputChannel(name) {
        calls.outputName = name;
        return { name };
      }
    }
  };

  Module._load = function (request, parent, isMain) {
    if (request === 'vscode') return fakeVscode;
    if (request === 'vscode-languageclient/node') {
      return { LanguageClient: class {
        constructor(...args) {
          calls.constructor = args;
          return fakeClient;
        }
      } };
    }
    return originalLoad.call(this, request, parent, isMain);
  };

  try {
    const { activate, deactivate } = require('./extension');
    await activate({ subscriptions: { push(...items) { subscriptions.push(...items); } } });
    const [id, name, serverOptions, clientOptions] = calls.constructor;
    assert.equal(id, 'zelyra');
    assert.equal(name, 'Zelyra Language Server');
    assert.equal(serverOptions.command, '/custom/bin/zelyra');
    assert.deepEqual(serverOptions.args, ['lsp']);
    assert.equal(serverOptions.options.cwd, path.resolve('/workspace/project'));
    assert.deepEqual(clientOptions.documentSelector, [{ scheme: 'file', language: 'zelyra' }]);
    assert.equal(calls.started, true);
    assert.equal(subscriptions.length, 2);
    await deactivate();
    assert.equal(calls.stopped, true);
  } finally {
    Module._load = originalLoad;
  }
});
