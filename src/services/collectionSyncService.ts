import { ApiRequest, Collection } from '../types';

function getBridge(): any {
  if (typeof window !== 'undefined' && (window as any).bapuBridge) {
    return (window as any).bapuBridge;
  }
  return null;
}

export const CollectionSyncService = {
  isAvailable(): boolean {
    return Boolean(getBridge()?.chooseFolder);
  },

  async chooseFolder(): Promise<string | null> {
    const bridge = getBridge();
    if (!bridge?.chooseFolder) return null;
    const res = await bridge.chooseFolder();
    return res?.success ? res.folderPath : null;
  },

  async writeToFolder(folderPath: string, collectionName: string, requests: ApiRequest[]): Promise<{ success: boolean; message?: string }> {
    const bridge = getBridge();
    if (!bridge?.writeCollectionFolder) {
      return { success: false, message: 'Folder sync is only available in the desktop app.' };
    }
    try {
      return await bridge.writeCollectionFolder({ folderPath, collectionName, requests });
    } catch (err: any) {
      return { success: false, message: err.message || 'Failed to write collection to folder' };
    }
  },

  async readFromFolder(folderPath: string): Promise<{ success: boolean; name?: string; requests?: ApiRequest[]; message?: string }> {
    const bridge = getBridge();
    if (!bridge?.readCollectionFolder) {
      return { success: false, message: 'Folder sync is only available in the desktop app.' };
    }
    try {
      return await bridge.readCollectionFolder(folderPath);
    } catch (err: any) {
      return { success: false, message: err.message || 'Failed to read collection from folder' };
    }
  }
};

export function collectionNeedsSync(collection: Collection): boolean {
  return Boolean(collection.folderPath);
}
