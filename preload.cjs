const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('bapuBridge', {
  dbTestConnection: (config) => ipcRenderer.invoke('db:test-connection', config),
  dbQuery: (params) => ipcRenderer.invoke('db:query', params),
  dbGetSchema: (config) => ipcRenderer.invoke('db:get-schema', config)
});
