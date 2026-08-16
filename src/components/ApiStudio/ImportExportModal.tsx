import React, { useState } from 'react';
import { 
  X, 
  Upload, 
  Download, 
  FileCode, 
  Copy, 
  Check, 
  FolderDown, 
  Sparkles,
  AlertCircle,
  FileText
} from 'lucide-react';
import { Collection } from '../../types';
import { exportToPostman, exportToOpenApi, importCollection } from '../../utils/collectionIO';

interface ImportExportModalProps {
  isOpen: boolean;
  onClose: () => void;
  collections: Collection[];
  onImportCollection: (collection: Collection) => void;
  selectedCollectionId?: string;
}

export const ImportExportModal: React.FC<ImportExportModalProps> = ({
  isOpen,
  onClose,
  collections,
  onImportCollection,
  selectedCollectionId
}) => {
  const [activeTab, setActiveTab] = useState<'import' | 'export'>('import');
  
  // Import State
  const [importText, setImportText] = useState('');
  const [importName, setImportName] = useState('');
  const [importError, setImportError] = useState<string | null>(null);
  const [importSuccess, setImportSuccess] = useState<string | null>(null);

  // Export State
  const [exportColId, setExportColId] = useState<string>(
    selectedCollectionId || (collections[0] ? collections[0].id : '')
  );
  const [exportFormat, setExportFormat] = useState<'postman' | 'openapi' | 'bapu'>('postman');
  const [copied, setCopied] = useState(false);

  if (!isOpen) return null;

  const currentExportCol = collections.find(c => c.id === exportColId) || collections[0];

  const getExportData = (): string => {
    if (!currentExportCol) return '{}';
    if (exportFormat === 'postman') {
      return exportToPostman(currentExportCol);
    } else if (exportFormat === 'openapi') {
      return exportToOpenApi(currentExportCol);
    } else {
      return JSON.stringify(currentExportCol, null, 2);
    }
  };

  const handleFileUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = (event) => {
      const content = event.target?.result as string;
      setImportText(content);
      if (!importName) {
        setImportName(file.name.replace(/\.[^/.]+$/, ''));
      }
    };
    reader.readAsText(file);
  };

  const handleExecuteImport = () => {
    setImportError(null);
    setImportSuccess(null);

    if (!importText.trim()) {
      setImportError('Please paste collection JSON/YAML or upload a file.');
      return;
    }

    try {
      const newCol = importCollection(importText, importName || undefined);
      onImportCollection(newCol);
      setImportSuccess(`Successfully imported "${newCol.name}" with ${newCol.requests.length} requests!`);
      setTimeout(() => {
        onClose();
        setImportText('');
        setImportName('');
        setImportSuccess(null);
      }, 900);
    } catch (err: any) {
      setImportError(err.message || 'Failed to import collection. Please check JSON syntax.');
    }
  };

  const handleDownloadExport = () => {
    if (!currentExportCol) return;
    const data = getExportData();
    const blob = new Blob([data], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    const ext = exportFormat === 'openapi' ? 'openapi.json' : 'postman_collection.json';
    a.href = url;
    a.download = `${currentExportCol.name.toLowerCase().replace(/[^a-z0-9]/g, '-')}.${ext}`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  };

  const handleCopyExport = () => {
    navigator.clipboard.writeText(getExportData());
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" onClick={(e) => e.stopPropagation()} style={{ maxWidth: '640px', width: '90%' }}>
        <div className="modal-header">
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <FolderDown size={18} color="#60a5fa" />
            <h3 style={{ fontSize: '15px', fontWeight: 700 }}>Collections Hub</h3>
          </div>
          <button onClick={onClose} className="sidebar-action-btn">
            <X size={16} />
          </button>
        </div>

        {/* Tab Switcher */}
        <div style={{ display: 'flex', borderBottom: '1px solid var(--border-subtle)', background: 'rgba(0,0,0,0.2)' }}>
          <button
            onClick={() => setActiveTab('import')}
            style={{
              flex: 1,
              padding: '10px',
              background: activeTab === 'import' ? 'var(--bg-card)' : 'transparent',
              border: 'none',
              borderBottom: activeTab === 'import' ? '2px solid #3b82f6' : 'none',
              color: activeTab === 'import' ? 'var(--text-main)' : 'var(--text-dim)',
              fontSize: '12px',
              fontWeight: 600,
              cursor: 'pointer',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              gap: '6px'
            }}
          >
            <Upload size={14} color="#60a5fa" />
            <span>Import Collection</span>
          </button>
          <button
            onClick={() => setActiveTab('export')}
            style={{
              flex: 1,
              padding: '10px',
              background: activeTab === 'export' ? 'var(--bg-card)' : 'transparent',
              border: 'none',
              borderBottom: activeTab === 'export' ? '2px solid #3b82f6' : 'none',
              color: activeTab === 'export' ? 'var(--text-main)' : 'var(--text-dim)',
              fontSize: '12px',
              fontWeight: 600,
              cursor: 'pointer',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              gap: '6px'
            }}
          >
            <Download size={14} color="#10b981" />
            <span>Export Collection</span>
          </button>
        </div>

        <div style={{ padding: '16px' }}>
          {activeTab === 'import' ? (
            <div>
              <div style={{ marginBottom: '12px' }}>
                <label style={{ display: 'block', fontSize: '11px', color: 'var(--text-dim)', marginBottom: '4px' }}>
                  Collection Name (Optional)
                </label>
                <input
                  type="text"
                  value={importName}
                  onChange={(e) => setImportName(e.target.value)}
                  placeholder="e.g. Stripe Billing API"
                  className="kv-input"
                />
              </div>

              {/* Drag and Drop File Upload Area */}
              <div 
                style={{
                  border: '2px dashed var(--border-subtle)',
                  borderRadius: 'var(--radius-md)',
                  padding: '18px',
                  textAlign: 'center',
                  marginBottom: '12px',
                  background: 'rgba(59, 130, 246, 0.03)',
                  cursor: 'pointer',
                  position: 'relative'
                }}
              >
                <input
                  type="file"
                  accept=".json,.yaml,.yml"
                  onChange={handleFileUpload}
                  style={{
                    position: 'absolute',
                    top: 0,
                    left: 0,
                    width: '100%',
                    height: '100%',
                    opacity: 0,
                    cursor: 'pointer'
                  }}
                />
                <Upload size={22} color="#60a5fa" style={{ margin: '0 auto 8px' }} />
                <div style={{ fontSize: '12px', fontWeight: 600, color: 'var(--text-main)' }}>
                  Click or drag files here to upload
                </div>
                <div style={{ fontSize: '11px', color: 'var(--text-dim)', marginTop: '4px' }}>
                  Supports <strong>Postman Collection v2.1</strong>, <strong>OpenAPI / Swagger 3.0</strong>, or <strong>Bapu JSON</strong>
                </div>
              </div>

              <div style={{ marginBottom: '12px' }}>
                <label style={{ display: 'block', fontSize: '11px', color: 'var(--text-dim)', marginBottom: '4px' }}>
                  Or paste JSON content directly:
                </label>
                <textarea
                  value={importText}
                  onChange={(e) => setImportText(e.target.value)}
                  placeholder='Paste Postman v2.1 JSON or OpenAPI 3.0 specification here...'
                  className="code-textarea"
                  style={{ height: '140px', fontSize: '11px' }}
                  spellCheck={false}
                />
              </div>

              {importError && (
                <div style={{
                  padding: '8px 12px',
                  borderRadius: 'var(--radius-sm)',
                  background: 'rgba(239, 68, 68, 0.1)',
                  border: '1px solid rgba(239, 68, 68, 0.3)',
                  color: '#f87171',
                  fontSize: '12px',
                  display: 'flex',
                  alignItems: 'center',
                  gap: '6px',
                  marginBottom: '12px'
                }}>
                  <AlertCircle size={14} />
                  <span>{importError}</span>
                </div>
              )}

              {importSuccess && (
                <div style={{
                  padding: '8px 12px',
                  borderRadius: 'var(--radius-sm)',
                  background: 'rgba(16, 185, 129, 0.1)',
                  border: '1px solid rgba(16, 185, 129, 0.3)',
                  color: '#34d399',
                  fontSize: '12px',
                  display: 'flex',
                  alignItems: 'center',
                  gap: '6px',
                  marginBottom: '12px'
                }}>
                  <Check size={14} />
                  <span>{importSuccess}</span>
                </div>
              )}

              <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '8px' }}>
                <button onClick={onClose} className="btn-secondary">
                  Cancel
                </button>
                <button onClick={handleExecuteImport} className="btn-send" style={{ fontSize: '12px' }}>
                  <Upload size={13} />
                  <span>Import Collection</span>
                </button>
              </div>
            </div>
          ) : (
            <div>
              <div style={{ display: 'flex', gap: '12px', marginBottom: '14px' }}>
                <div style={{ flex: 1 }}>
                  <label style={{ display: 'block', fontSize: '11px', color: 'var(--text-dim)', marginBottom: '4px' }}>
                    Select Collection
                  </label>
                  <select
                    value={exportColId}
                    onChange={(e) => setExportColId(e.target.value)}
                    className="kv-input"
                  >
                    {collections.map(c => (
                      <option key={c.id} value={c.id}>
                        {c.name} ({c.requests.length} requests)
                      </option>
                    ))}
                  </select>
                </div>

                <div style={{ width: '200px' }}>
                  <label style={{ display: 'block', fontSize: '11px', color: 'var(--text-dim)', marginBottom: '4px' }}>
                    Export Format
                  </label>
                  <select
                    value={exportFormat}
                    onChange={(e) => setExportFormat(e.target.value as any)}
                    className="kv-input"
                  >
                    <option value="postman">Postman v2.1 Compatible (.json)</option>
                    <option value="openapi">OpenAPI 3.0 Specification (.json)</option>
                    <option value="bapu">Bapu Studio Native (.json)</option>
                  </select>
                </div>
              </div>

              <div style={{ marginBottom: '14px' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '4px' }}>
                  <label style={{ fontSize: '11px', color: 'var(--text-dim)' }}>
                    Exported Payload Preview
                  </label>
                  <button onClick={handleCopyExport} className="sidebar-action-btn" title="Copy to Clipboard">
                    {copied ? <Check size={13} color="#10b981" /> : <Copy size={13} />}
                  </button>
                </div>
                <pre style={{
                  height: '180px',
                  background: '#070a10',
                  border: '1px solid var(--border-subtle)',
                  borderRadius: 'var(--radius-sm)',
                  padding: '10px',
                  color: '#38bdf8',
                  fontFamily: 'var(--font-mono)',
                  fontSize: '11px',
                  lineHeight: '1.5',
                  overflow: 'auto',
                  margin: 0
                }}>
                  {getExportData()}
                </pre>
              </div>

              <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '8px' }}>
                <button onClick={onClose} className="btn-secondary">
                  Close
                </button>
                <button onClick={handleDownloadExport} className="btn-send" style={{ fontSize: '12px' }}>
                  <Download size={13} />
                  <span>Download Collection (.json)</span>
                </button>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
