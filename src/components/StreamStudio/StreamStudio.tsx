import React, { useState, useEffect, useRef } from 'react';
import { 
  Radio, 
  Send, 
  Play, 
  Square, 
  Sparkles, 
  Clock, 
  ArrowUpRight, 
  ArrowDownLeft, 
  Trash2,
  Bot
} from 'lucide-react';

interface StreamMessage {
  id: string;
  type: 'sent' | 'received' | 'system';
  content: string;
  timestamp: string;
  latencyMs?: number;
}

export const StreamStudio: React.FC = () => {
  const [streamType, setStreamType] = useState<'sse' | 'ws'>('sse');
  const [url, setUrl] = useState('https://api.example.com/v1/chat/completions/stream');
  const [isConnected, setIsConnected] = useState(false);
  const [outMessage, setOutMessage] = useState('{"type": "subscribe", "channel": "live_feed"}');
  const [messages, setMessages] = useState<StreamMessage[]>([
    {
      id: 'm-1',
      type: 'system',
      content: 'Ready to connect to real-time streaming endpoint.',
      timestamp: '00:00:00'
    }
  ]);

  const intervalRef = useRef<any>(null);

  const handleToggleConnect = () => {
    if (isConnected) {
      setIsConnected(false);
      if (intervalRef.current) clearInterval(intervalRef.current);
      setMessages(prev => [
        ...prev,
        {
          id: `m-${Date.now()}`,
          type: 'system',
          content: 'Disconnected from stream.',
          timestamp: new Date().toLocaleTimeString()
        }
      ]);
    } else {
      setIsConnected(true);
      setMessages(prev => [
        ...prev,
        {
          id: `m-${Date.now()}`,
          type: 'system',
          content: `Connected to ${url} (${streamType.toUpperCase()})`,
          timestamp: new Date().toLocaleTimeString()
        }
      ]);

      if (streamType === 'sse') {
        // Simulate real-time token stream (like OpenAI/Claude streaming tokens)
        const tokens = [
          "Hello! ", "I ", "am ", "streaming ", "tokens ", "in ", "real-time ", "directly ", 
          "into ", "Bapu ", "Studio. ", "Zero ", "latency ", "and ", "100% ", "offline ", "privacy!"
        ];
        let idx = 0;
        intervalRef.current = setInterval(() => {
          if (idx < tokens.length) {
            const token = tokens[idx];
            setMessages(prev => [
              ...prev,
              {
                id: `m-token-${Date.now()}-${idx}`,
                type: 'received',
                content: `data: {"id":"chatcmpl-9","choices":[{"delta":{"content":"${token}"}}]}`,
                timestamp: new Date().toLocaleTimeString(),
                latencyMs: 18 + Math.floor(Math.random() * 12)
              }
            ]);
            idx++;
          } else {
            clearInterval(intervalRef.current);
          }
        }, 120);
      }
    }
  };

  const handleSendMessage = () => {
    if (!outMessage.trim()) return;
    setMessages(prev => [
      ...prev,
      {
        id: `m-${Date.now()}`,
        type: 'sent',
        content: outMessage,
        timestamp: new Date().toLocaleTimeString()
      }
    ]);
    
    // Simulate echo reply for WebSockets
    setTimeout(() => {
      setMessages(prev => [
        ...prev,
        {
          id: `m-rep-${Date.now()}`,
          type: 'received',
          content: `ACK: Received frame "${outMessage.substring(0, 30)}..."`,
          timestamp: new Date().toLocaleTimeString(),
          latencyMs: 24
        }
      ]);
    }, 180);
  };

  const handleClear = () => {
    setMessages([]);
  };

  useEffect(() => {
    return () => {
      if (intervalRef.current) clearInterval(intervalRef.current);
    };
  }, []);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', overflow: 'hidden' }}>
      {/* Top Connection Bar */}
      <div className="request-bar-container">
        <select
          value={streamType}
          onChange={(e) => {
            setStreamType(e.target.value as any);
            setUrl(e.target.value === 'sse' 
              ? 'https://api.example.com/v1/chat/completions/stream' 
              : 'wss://echo.websocket.events');
          }}
          className="method-select-dropdown"
        >
          <option value="sse">SSE Stream</option>
          <option value="ws">WebSocket</option>
        </select>

        <div className="url-input-wrapper">
          <input
            type="text"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            placeholder="ws:// or https:// stream URL"
            className="url-input"
          />
        </div>

        <button
          onClick={handleToggleConnect}
          className={isConnected ? "btn-secondary" : "btn-send"}
          style={{
            background: isConnected ? 'rgba(239, 68, 68, 0.2)' : undefined,
            color: isConnected ? '#ef4444' : undefined,
            borderColor: isConnected ? 'rgba(239, 68, 68, 0.4)' : undefined
          }}
        >
          {isConnected ? <Square size={13} /> : <Play size={13} />}
          <span>{isConnected ? 'Disconnect' : 'Connect'}</span>
        </button>
      </div>

      {/* Stream Split View */}
      <div className="split-workspace-pane">
        {/* Left: WebSocket Frame Composer */}
        {streamType === 'ws' && (
          <div className="request-config-panel" style={{ maxWidth: '380px' }}>
            <div className="panel-tab-header">
              <span style={{ fontSize: '12px', fontWeight: 600 }}>Message Payload</span>
            </div>
            <div style={{ flex: 1, padding: '12px', display: 'flex', flexDirection: 'column' }}>
              <textarea
                value={outMessage}
                onChange={(e) => setOutMessage(e.target.value)}
                placeholder="Message to send..."
                className="code-textarea"
                style={{ flex: 1, background: 'var(--bg-input)', padding: '8px', borderRadius: 'var(--radius-sm)' }}
              />
              <button
                onClick={handleSendMessage}
                disabled={!isConnected}
                className="btn-send"
                style={{ marginTop: '10px', justifyContent: 'center' }}
              >
                <Send size={13} />
                <span>Send Frame</span>
              </button>
            </div>
          </div>
        )}

        {/* Right: Live Event Timeline */}
        <div className="response-viewer-panel" style={{ flex: 1 }}>
          <div className="panel-tab-header">
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <Radio size={14} color={isConnected ? '#10b981' : 'var(--text-dim)'} />
              <span style={{ fontSize: '12px', fontWeight: 600 }}>
                Live Stream Event Timeline ({messages.length})
              </span>
            </div>

            <button onClick={handleClear} className="sidebar-action-btn" title="Clear Stream Logs">
              <Trash2 size={13} />
            </button>
          </div>

          <div className="editor-container" style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
            {messages.map((msg) => (
              <div
                key={msg.id}
                style={{
                  display: 'flex',
                  alignItems: 'flex-start',
                  gap: '8px',
                  padding: '6px 10px',
                  borderRadius: 'var(--radius-sm)',
                  background: msg.type === 'sent' 
                    ? 'rgba(59, 130, 246, 0.08)' 
                    : msg.type === 'received' 
                    ? 'rgba(16, 185, 129, 0.08)' 
                    : 'rgba(255, 255, 255, 0.03)',
                  border: '1px solid rgba(255, 255, 255, 0.04)',
                  fontSize: '12px',
                  fontFamily: 'var(--font-mono)'
                }}
              >
                <span style={{ marginTop: '2px' }}>
                  {msg.type === 'sent' && <ArrowUpRight size={13} color="#3b82f6" />}
                  {msg.type === 'received' && <ArrowDownLeft size={13} color="#10b981" />}
                  {msg.type === 'system' && <Clock size={13} color="var(--text-dim)" />}
                </span>

                <div style={{ flex: 1, wordBreak: 'break-all' }}>
                  <div style={{ display: 'flex', justifyContent: 'space-between', color: 'var(--text-dim)', fontSize: '10px', marginBottom: '2px' }}>
                    <span>{msg.type.toUpperCase()}</span>
                    <span>{msg.timestamp} {msg.latencyMs ? `• ${msg.latencyMs}ms` : ''}</span>
                  </div>
                  <span style={{ color: msg.type === 'received' ? '#38bdf8' : msg.type === 'sent' ? '#f1f5f9' : 'var(--text-muted)' }}>
                    {msg.content}
                  </span>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
};
