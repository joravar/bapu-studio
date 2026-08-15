import React, { useState } from 'react';
import { X, Copy, Check, Code2, Terminal, Cpu } from 'lucide-react';
import { ApiRequest, Environment } from '../../types';
import { generateCodeSnippet, SupportedLanguage } from '../../utils/codeGenerators';

interface CodeSnippetModalProps {
  isOpen: boolean;
  onClose: () => void;
  activeRequest: ApiRequest;
  activeEnv: Environment;
}

export const CodeSnippetModal: React.FC<CodeSnippetModalProps> = ({
  isOpen,
  onClose,
  activeRequest,
  activeEnv
}) => {
  const [selectedLang, setSelectedLang] = useState<SupportedLanguage>('javascript_fetch');
  const [copied, setCopied] = useState(false);

  if (!isOpen) return null;

  const snippet = generateCodeSnippet(activeRequest, activeEnv, selectedLang);

  const handleCopy = () => {
    navigator.clipboard.writeText(snippet);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const languages: { id: SupportedLanguage; label: string; tag: string }[] = [
    { id: 'javascript_fetch', label: 'JavaScript (Fetch)', tag: 'JS/TS' },
    { id: 'javascript_axios', label: 'JavaScript (Axios)', tag: 'JS/TS' },
    { id: 'python_requests', label: 'Python (Requests)', tag: 'Python' },
    { id: 'python_httpx', label: 'Python (HTTPX Async)', tag: 'Python' },
    { id: 'go_http', label: 'Go (net/http)', tag: 'Go' },
    { id: 'rust_reqwest', label: 'Rust (reqwest)', tag: 'Rust' },
    { id: 'shell_curl', label: 'cURL Command', tag: 'Shell' }
  ];

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" style={{ maxWidth: '780px', maxHeight: '85vh', display: 'flex', flexDirection: 'column' }} onClick={(e) => e.stopPropagation()}>
        {/* Header */}
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '16px' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <Code2 size={20} color="#3b82f6" />
            <h2 style={{ fontSize: '18px', fontWeight: 700, color: '#fff' }}>
              Generate Client Code
            </h2>
          </div>

          <button onClick={onClose} className="sidebar-action-btn" style={{ padding: '4px' }}>
            <X size={16} />
          </button>
        </div>

        {/* Language Tabs */}
        <div style={{ display: 'flex', gap: '6px', overflowX: 'auto', paddingBottom: '8px', borderBottom: '1px solid var(--border-subtle)' }}>
          {languages.map((lang) => (
            <button
              key={lang.id}
              onClick={() => setSelectedLang(lang.id)}
              className={`subtab-btn ${selectedLang === lang.id ? 'active' : ''}`}
              style={{ padding: '6px 12px', fontSize: '12px', whiteSpace: 'nowrap' }}
            >
              {lang.label}
            </button>
          ))}
        </div>

        {/* Code Viewer */}
        <div style={{ position: 'relative', marginTop: '12px', flex: 1, minHeight: '280px', background: '#070a10', border: '1px solid var(--border-subtle)', borderRadius: 'var(--radius-md)', padding: '16px', overflow: 'auto' }}>
          <button
            onClick={handleCopy}
            style={{
              position: 'absolute',
              top: '12px',
              right: '12px',
              display: 'flex',
              alignItems: 'center',
              gap: '6px',
              background: 'rgba(255,255,255,0.08)',
              border: '1px solid var(--border-subtle)',
              color: '#fff',
              padding: '5px 10px',
              borderRadius: 'var(--radius-sm)',
              fontSize: '11px',
              cursor: 'pointer',
              fontWeight: 600
            }}
          >
            {copied ? <Check size={13} color="#10b981" /> : <Copy size={13} />}
            <span>{copied ? 'Copied!' : 'Copy Code'}</span>
          </button>

          <pre style={{
            fontFamily: 'var(--font-mono)',
            fontSize: '12px',
            color: '#38bdf8',
            lineHeight: '1.6',
            whiteSpace: 'pre-wrap'
          }}>
            {snippet}
          </pre>
        </div>
      </div>
    </div>
  );
};
