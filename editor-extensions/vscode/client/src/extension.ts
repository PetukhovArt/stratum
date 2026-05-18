import * as vscode from 'vscode'
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from 'vscode-languageclient/node'

let client: LanguageClient | undefined

export function activate(context: vscode.ExtensionContext): void {
  const config = vscode.workspace.getConfiguration('stratum')
  const serverPath = config.get<string>('serverPath', 'stratum-lsp')

  const serverOptions: ServerOptions = {
    run: { command: serverPath, transport: TransportKind.stdio },
    debug: { command: serverPath, transport: TransportKind.stdio },
  }

  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: 'file', language: 'typescript' },
      { scheme: 'file', language: 'typescriptreact' },
      { scheme: 'file', language: 'javascript' },
      { scheme: 'file', language: 'javascriptreact' },
      { scheme: 'file', language: 'vue' },
    ],
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher('**/stratum.config.jsonc'),
    },
  }

  client = new LanguageClient('stratum-lsp', 'Stratum LSP', serverOptions, clientOptions)
  context.subscriptions.push({ dispose: () => client?.stop() })
  void client.start()
}

export function deactivate(): Thenable<void> | undefined {
  return client?.stop()
}
