import React from 'react';
import { 
  Zap, 
  ChevronDown, 
  Heart, 
  GitBranch, 
  Search, 
  Minus, 
  Square, 
  X,
  ShieldCheck
} from 'lucide-react';
import { Environment } from '../types';

interface HeaderProps {
  environments: Environment[];
  activeEnv: Environment;
  onSelectEnv: (env: Environment) => void;
}

export const Header: React.FC<HeaderProps> = ({
  environments,
  activeEnv,
  onSelectEnv
}) => {
  return (
    <header className="nexus-header">
      {/* Left: Brand Identity & Git Status */}
      <div className="header-left">
        <div className="brand-badge">
          <div className="brand-icon">
            <Zap size={15} />
          </div>
          <span>Bapu Studio</span>
          <span className="oss-tag">AGPLv3</span>
        </div>

        <div style={{ width: '1px', height: '16px', background: 'var(--border-subtle)' }} />

        <div style={{ display: 'flex', alignItems: 'center', gap: '6px', color: 'var(--text-dim)', fontSize: '11px' }}>
          <GitBranch size={13} color="#10b981" />
          <span style={{ color: 'var(--text-muted)', fontFamily: 'var(--font-mono)' }}>main</span>
          <span style={{ color: 'var(--text-dim)' }}>(3 collections synced)</span>
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
              <option key={env.id} value={env.id} style={{ background: '#0f1522', color: '#fff' }}>
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
        <a 
          href="https://github.com/sponsors/joravar"
          target="_blank"
          rel="noopener noreferrer"
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

        <div style={{ display: 'flex', alignItems: 'center', gap: '4px', marginLeft: '8px' }}>
          <button className="sidebar-action-btn" title="Minimize Window">
            <Minus size={12} />
          </button>
          <button className="sidebar-action-btn" title="Maximize Window">
            <Square size={10} />
          </button>
          <button className="sidebar-action-btn" title="Close Window">
            <X size={12} />
          </button>
        </div>
      </div>
    </header>
  );
};
