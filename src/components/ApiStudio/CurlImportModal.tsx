import React, { useState } from 'react';
import { X, Terminal, ArrowRight, Sparkles } from 'lucide-react';
import { ApiRequest } from '../../types';
import { parseCurlCommand } from '../../utils/curlParser';

interface CurlImportModalProps {
  isOpen: boolean;
  onClose: () => void;
  onImport: (parsedReq: Partial<ApiRequest>) => void;
}

export const CurlImportModal: React.FC<CurlImportModalProps> = ({
  isOpen,
  onClose,
  onImport
}) => {
  const [curlText, setCurlText] = useState(`curl -X POST https://api.example.com/v1/users \\
  -H "Content-Type: application/json" \\
  -H "Authorization: Bearer {{AUTH_TOKEN}}" \\
  -d '{"name": "Dev User", "role": "admin"}'`);

  if (!isOpen) return null;

  const handleImport = () => {
    if (!curlText.trim()) return;
    const parsed = parseCurlCommand(curlText);
    onImport(parsed);
    onClose();
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" style={{ maxWidth: '640px' }} onClick={(e) => e.stopPropagation()}>
        {/* Header */}
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '16px' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <Terminal size={20} color="#06b6d4" />
            <h2 style={{ fontSize: '18px', fontWeight: 700, color: '#fff' }}>
              Import from cURL
            </h2>
          </div>

          <button onClick={onClose} className="sidebar-action-btn" style={{ padding: '4px' }}>
            <X size={16} />
          </button>
        </div>

        <p style={{ color: 'var(--text-muted)', fontSize: '12px', marginBottom: '12px' }}>
          Paste any raw <code>curl</code> command copied from your browser DevTools Network tab, API documentation, or terminal:
        </p>

        <textarea
          value={curlText}
          onChange={(e) => setCurlText(e.target.value)}
          placeholder="Paste curl command here..."
          className="code-textarea"
          style={{
            height: '180px',
            background: '#070a10',
            border: '1px solid var(--border-subtle)',
            borderRadius: 'var(--radius-md)',
            padding: '12px',
            fontSize: '12px',
            color: '#a5f3fc'
          }}
          spellCheck={false}
        />

        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '10px', marginTop: '16px' }}>
          <button onClick={onClose} className="btn-secondary">
            Cancel
          </button>
          <button onClick={handleImport} className="btn-send" style={{ padding: '8px 18px' }}>
            <span>Import Request</span>
            <ArrowRight size={14} />
          </button>
        </div>
      </div>
    </div>
  );
};
