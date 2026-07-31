const vscode = require("vscode");
const {
  LanguageClient,
  TransportKind,
} = require("vscode-languageclient/node");

let client;

function createClient() {
  const configuration = vscode.workspace.getConfiguration("luna");
  const serverPath = configuration.get("server.path", "luna-lsp");
  const compilerPath = configuration.get("compiler.path", "");
  const environment = { ...process.env };
  if (compilerPath) environment.LUNA_BIN = compilerPath;

  const executable = {
    command: serverPath,
    transport: TransportKind.stdio,
    options: { env: environment },
  };
  return new LanguageClient(
    "luna",
    "Luna Language Server",
    { run: executable, debug: executable },
    {
      documentSelector: [{ scheme: "file", language: "luna" }],
      initializationOptions: compilerPath ? { lunaPath: compilerPath } : {},
    },
  );
}

async function startClient() {
  if (client) return;
  client = createClient();
  await client.start();
}

async function stopClient() {
  if (!client) return;
  const running = client;
  client = undefined;
  await running.stop();
}

async function activate(context) {
  context.subscriptions.push(
    vscode.commands.registerCommand("luna.restartLanguageServer", async () => {
      await stopClient();
      await startClient();
    }),
  );
  await startClient();
}

async function deactivate() {
  await stopClient();
}

module.exports = { activate, deactivate };
