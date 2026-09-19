import React from 'react';
import {
  ChevronDown,
  Heart,
  FolderOpen,
  Search,
  Sun,
  Moon
} from 'lucide-react';
import { Collection, Environment } from '../types';
import { Theme } from '../utils/theme';
import logoUrl from '../assets/logo.png';

interface HeaderProps {
  environments: Environment[];
  activeEnv: Environment;
  onSelectEnv: (env: Environment) => void;
  collections: Collection[];
  theme: Theme;
  onToggleTheme: () => void;
}

export const Header: React.FC<HeaderProps> = ({
  environments,
  activeEnv,
  onSelectEnv,
  collections,
  theme,
  onToggleTheme
}) => {
  const requestCount = collections.reduce((sum, c) => sum + c.requests.length, 0);
  return (
    <header className="nexus-header">
      {/* Left: Brand Identity & Git Status */}
      <div className="header-left">
        <div className="brand-badge">
          <img src={logoUrl} alt="Bapu Studio" className="brand-logo-img" />
          <span>Bapu Studio</span>
          <span className="oss-tag">AGPLv3</span>
        </div>

        <div style={{ width: '1px', height: '16px', background: 'var(--border-subtle)' }} />

        <div style={{ display: 'flex', alignItems: 'center', gap: '6px', color: 'var(--text-dim)', fontSize: '11px' }}>
          <FolderOpen size={13} color="#10b981" />
          <span style={{ color: 'var(--text-muted)' }}>
            {collections.length} collection{collections.length !== 1 ? 's' : ''}
          </span>
          <span style={{ color: 'var(--text-dim)' }}>({requestCount} request{requestCount !== 1 ? 's' : ''})</span>
        </div>
      </div>

      {/* Center: Environment Switcher & Quick Search */}
      <div className="header-center">
        <div style={{ position: 'relative' }}>
          <select 
            value={activeEnv.id} 
            onChange={(e) => {
              const found = environments.find(env => env.id === e.target.value);
              if (found) onSelectEnv(found);
            }}
            className="env-selector"
            style={{ appearance: 'none', paddingRight: '26px', outline: 'none' }}
          >
            {environments.map(env => (
              <option key={env.id} value={env.id} style={{ background: 'var(--bg-card)', color: 'var(--text-main)' }}>
                🟢 {env.name}
              </option>
            ))}
          </select>
          <ChevronDown size={12} style={{ position: 'absolute', right: '8px', top: '50%', transform: 'translateY(-50%)', pointerEvents: 'none', color: 'var(--text-dim)' }} />
        </div>

        <div style={{
          display: 'flex',
          alignItems: 'center',
          gap: '6px',
          background: 'var(--bg-input)',
          border: '1px solid var(--border-subtle)',
          padding: '4px 10px',
          borderRadius: 'var(--radius-md)',
          color: 'var(--text-dim)',
          fontSize: '11px',
          cursor: 'pointer'
        }}>
          <Search size={12} />
          <span>Quick Find...</span>
          <kbd style={{
            background: 'var(--bg-card)',
            padding: '1px 5px',
            borderRadius: '4px',
            border: '1px solid var(--border-subtle)',
            fontFamily: 'var(--font-mono)',
            fontSize: '10px'
          }}>Ctrl+K</kbd>
        </div>
      </div>

      {/* Right: GitHub Sponsors & Window Controls */}
      <div className="header-right">
        <button
          onClick={onToggleTheme}
          title={theme === 'dark' ? 'Switch to light theme' : 'Switch to dark theme'}
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            width: '28px',
            height: '28px',
            background: 'var(--bg-input)',
            border: '1px solid var(--border-subtle)',
            borderRadius: 'var(--radius-md)',
            color: 'var(--text-muted)',
            cursor: 'pointer'
          }}
        >
          {theme === 'dark' ? <Sun size={14} /> : <Moon size={14} />}
        </button>

        <a
          href="https://github.com/sponsors/joravar"
          target="_blank"
          rel="noopener noreferrer"
          onClick={(e) => {
            if (typeof window !== 'undefined' && (window as any).bapuBridge?.openExternal) {
              e.preventDefault();
              (window as any).bapuBridge.openExternal('https://github.com/sponsors/joravar');
            }
          }}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '6px',
            background: 'rgba(236, 72, 153, 0.12)',
            border: '1px solid rgba(236, 72, 153, 0.3)',
            color: '#f472b6',
            fontSize: '11px',
            fontWeight: 700,
            padding: '4px 10px',
            borderRadius: 'var(--radius-md)',
            textDecoration: 'none',
            transition: 'all 0.15s ease'
          }}
          title="Support Bapu Studio development on GitHub Sponsors"
        >
          <Heart size={12} fill="#ec4899" color="#ec4899" />
          <span>Sponsor</span>
        </a>
      </div>
    </header>
  );
};
