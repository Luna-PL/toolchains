const vscode = require("vscode");
const { spawn } = require("node:child_process");
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
    vscode.languages.registerDocumentFormattingEditProvider("luna", {
      provideDocumentFormattingEdits: formatDocument,
    }),
  );
  await startClient();
}

async function deactivate() {
  await stopClient();
}

function formatDocument(document, cancellationToken) {
  if (cancellationToken.isCancellationRequested) {
    return Promise.reject(new vscode.CancellationError());
  }
  const formatterPath = vscode.workspace
    .getConfiguration("luna")
    .get("formatter.path", "luna-fmt");
  return new Promise((resolve, reject) => {
    const source = document.getText();
    const child = spawn(formatterPath, ["-"], {
      stdio: ["pipe", "pipe", "pipe"],
      windowsHide: true,
    });
    const stdout = [];
    const stderr = [];
    let settled = false;
    let cancellation;
    const finish = (callback) => {
      if (settled) return;
      settled = true;
      cancellation?.dispose();
      callback();
    };
    child.stdout.on("data", (chunk) => stdout.push(chunk));
    child.stderr.on("data", (chunk) => stderr.push(chunk));
    child.on("error", (error) => finish(() => reject(error)));
    child.stdin.on("error", (error) => finish(() => reject(error)));
    child.on("close", (status) => {
      if (status !== 0) {
        const message =
          Buffer.concat(stderr).toString("utf8").trim() ||
          `luna-fmt exited with status ${status}`;
        finish(() => reject(new Error(message)));
        return;
      }
      const formatted = Buffer.concat(stdout).toString("utf8");
      if (formatted === source) {
        finish(() => resolve([]));
        return;
      }
      const fullDocument = new vscode.Range(
        document.positionAt(0),
        document.positionAt(source.length),
      );
      finish(() =>
        resolve([vscode.TextEdit.replace(fullDocument, formatted)]),
      );
    });
    cancellation = cancellationToken.onCancellationRequested(() => {
      child.kill();
      finish(() => reject(new vscode.CancellationError()));
    });
    child.stdin.end(source, "utf8");
  });
}

module.exports = { activate, deactivate };
