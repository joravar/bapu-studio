const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('bapuBridge', {
  dbTestConnection: (config) => ipcRenderer.invoke('db:test-connection', config),
  dbQuery: (params) => ipcRenderer.invoke('db:query', params),
  dbGetSchema: (config) => ipcRenderer.invoke('db:get-schema', config),
  dbDisconnect: (id) => ipcRenderer.invoke('db:disconnect', id),
  openExternal: (url) => ipcRenderer.invoke('app:open-external', url),
  secureEncrypt: (text) => ipcRenderer.sendSync('secure:encrypt-sync', text),
  secureDecrypt: (text) => ipcRenderer.sendSync('secure:decrypt-sync', text),
  chooseFolder: () => ipcRenderer.invoke('fs:choose-folder'),
  writeCollectionFolder: (payload) => ipcRenderer.invoke('fs:write-collection-folder', payload),
  readCollectionFolder: (folderPath) => ipcRenderer.invoke('fs:read-collection-folder', folderPath)
});
