const { useState, useEffect, useRef, useCallback } = React;

const API_BASE = '';

function formatTs(ts) {
  const d = new Date(ts);
  return d.toLocaleString('sv-SE', {
    year: 'numeric', month: '2-digit', day: '2-digit',
    hour: '2-digit', minute: '2-digit', second: '2-digit',
    fractionalSecondDigits: 3,
  });
}

function formatDuration(startTime, endTime) {
  const start = new Date(startTime).getTime();
  const end = new Date(endTime).getTime();
  const ms = end - start;
  if (ms < 1) return '<1ms';
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(2)}s`;
}

function levelClass(level) {
  return (level || '').toLowerCase();
}

// ---- Logs View ----

function LogsView() {
  const [logs, setLogs] = useState([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState('');
  const [labels, setLabels] = useState([]);
  const [labelValues, setLabelValues] = useState({});
  const [selectedLabels, setSelectedLabels] = useState({});
  const [limit, setLimit] = useState(200);
  const [autoRefresh, setAutoRefresh] = useState(false);
  const viewerRef = useRef(null);

  const fetchLogs = useCallback(async () => {
    try {
      const res = await fetch(`${API_BASE}/api/logs?limit=${limit}`);
      const data = await res.json();
      setLogs(data);
    } catch (e) {
      console.error('Failed to fetch logs:', e);
    } finally {
      setLoading(false);
    }
  }, [limit]);

  const fetchLabels = useCallback(async () => {
    try {
      const res = await fetch(`${API_BASE}/rgrab/api/v1/labels`);
      const data = await res.json();
      if (data.status === 'success') {
        setLabels(data.data || []);
        const vals = {};
        for (const name of (data.data || [])) {
          const vRes = await fetch(`${API_BASE}/rgrab/api/v1/label/${name}/values`);
          const vData = await vRes.json();
          if (vData.status === 'success') {
            vals[name] = vData.data || [];
          }
        }
        setLabelValues(vals);
      }
    } catch (e) {
      console.error('Failed to fetch labels:', e);
    }
  }, []);

  useEffect(() => {
    fetchLogs();
    fetchLabels();
  }, [fetchLogs, fetchLabels]);

  useEffect(() => {
    if (!autoRefresh) return;
    const id = setInterval(fetchLogs, 2000);
    return () => clearInterval(id);
  }, [autoRefresh, fetchLogs]);

  const toggleLabel = (name, value) => {
    setSelectedLabels(prev => {
      const next = { ...prev };
      if (next[name] === value) {
        delete next[name];
      } else {
        next[name] = value;
      }
      return next;
    });
  };

  const filteredLogs = logs.filter(log => {
    for (const [k, v] of Object.entries(selectedLabels)) {
      if (k === 'level') {
        if ((log.level || '').toLowerCase() !== v.toLowerCase()) return false;
      } else if ((log.labels || {})[k] !== v) {
        return false;
      }
    }
    if (search) {
      const s = search.toLowerCase();
      if (!(log.message || '').toLowerCase().includes(s)) return false;
    }
    return true;
  });

  const stats = filteredLogs.reduce((acc, l) => {
    const lvl = (l.level || 'unknown').toLowerCase();
    acc[lvl] = (acc[lvl] || 0) + 1;
    return acc;
  }, {});

  if (loading) {
    return React.createElement('div', { className: 'loading' },
      React.createElement('div', { className: 'spinner' }),
      'Loading logs...'
    );
  }

  return React.createElement('div', { className: 'main' },
    // Sidebar
    React.createElement('div', { className: 'sidebar' },
      labels.map(name =>
        React.createElement('div', { key: name, className: 'sidebar-section' },
          React.createElement('div', { className: 'sidebar-title' }, name),
          (labelValues[name] || []).map(val =>
            React.createElement('div', {
              key: val,
              className: `sidebar-item ${selectedLabels[name] === val ? 'active' : ''}`,
              onClick: () => toggleLabel(name, val),
            },
              React.createElement('span', null, val),
              name === 'level' ? React.createElement('span', {
                className: 'sidebar-badge',
                style: { color: `var(--${levelClass(val)}, var(--text-muted))` }
              }) : null
            )
          )
        )
      )
    ),
    // Content
    React.createElement('div', { className: 'content' },
      // Toolbar
      React.createElement('div', { className: 'toolbar' },
        React.createElement('input', {
          className: 'search-input',
          placeholder: 'Search logs...',
          value: search,
          onChange: e => setSearch(e.target.value),
        }),
        React.createElement('select', {
          className: 'btn',
          value: limit,
          onChange: e => setLimit(Number(e.target.value)),
        },
          [100, 200, 500, 1000].map(n =>
            React.createElement('option', { key: n, value: n }, `${n} lines`)
          )
        ),
        React.createElement('button', {
          className: `btn ${autoRefresh ? 'btn-primary' : ''}`,
          onClick: () => setAutoRefresh(p => !p),
        }, autoRefresh ? 'Live' : 'Auto'),
        React.createElement('button', {
          className: 'btn',
          onClick: fetchLogs,
        }, 'Refresh'),
      ),
      // Log lines
      React.createElement('div', { className: 'log-viewer', ref: viewerRef },
        filteredLogs.length === 0
          ? React.createElement('div', { className: 'empty-state' }, 'No logs found')
          : filteredLogs.map((log, i) =>
              React.createElement('div', {
                key: i,
                className: `log-line level-${levelClass(log.level)}`,
              },
                React.createElement('span', { className: 'log-ts' }, formatTs(log.timestamp)),
                React.createElement('span', {
                  className: `log-level ${levelClass(log.level)}`,
                }, (log.level || '').toUpperCase()),
                React.createElement('span', { className: 'log-msg' }, log.message),
                React.createElement('span', { className: 'log-labels' },
                  Object.entries(log.labels || {}).slice(0, 3).map(([k, v]) =>
                    React.createElement('span', { key: k, className: 'log-label' }, `${k}=${v}`)
                  )
                ),
              )
            )
      ),
      // Stats
      React.createElement('div', { className: 'stats-bar' },
        React.createElement('span', { className: 'stat-item' }, `${filteredLogs.length} lines`),
        ['error', 'warn', 'info', 'debug'].map(lvl =>
          stats[lvl] ? React.createElement('span', { key: lvl, className: 'stat-item' },
            React.createElement('span', { className: `stat-dot ${lvl}` }),
            `${lvl}: ${stats[lvl]}`
          ) : null
        )
      )
    )
  );
}

// ---- Traces View ----

function TracesView() {
  const [traces, setTraces] = useState([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState('');
  const [expanded, setExpanded] = useState(null);
  const [traceSpans, setTraceSpans] = useState({});
  const [limit, setLimit] = useState(100);

  const fetchTraces = useCallback(async () => {
    try {
      const res = await fetch(`${API_BASE}/api/traces?limit=${limit}`);
      const data = await res.json();

      // Group by trace_id, keep first span as representative
      const grouped = {};
      for (const span of data) {
        if (!grouped[span.trace_id]) {
          grouped[span.trace_id] = span;
        }
      }
      setTraces(Object.values(grouped));
    } catch (e) {
      console.error('Failed to fetch traces:', e);
    } finally {
      setLoading(false);
    }
  }, [limit]);

  useEffect(() => { fetchTraces(); }, [fetchTraces]);

  const toggleTrace = async (traceId) => {
    if (expanded === traceId) {
      setExpanded(null);
      return;
    }
    setExpanded(traceId);
    if (!traceSpans[traceId]) {
      try {
        const res = await fetch(`${API_BASE}/api/traces?trace_id=${traceId}`);
        const data = await res.json();
        setTraceSpans(prev => ({ ...prev, [traceId]: data }));
      } catch (e) {
        console.error('Failed to fetch trace spans:', e);
      }
    }
  };

  const filtered = traces.filter(t => {
    if (!search) return true;
    const s = search.toLowerCase();
    return (t.operation_name || '').toLowerCase().includes(s)
      || (t.service_name || '').toLowerCase().includes(s)
      || (t.trace_id || '').toLowerCase().includes(s);
  });

  if (loading) {
    return React.createElement('div', { className: 'loading' },
      React.createElement('div', { className: 'spinner' }),
      'Loading traces...'
    );
  }

  return React.createElement('div', { className: 'main' },
    React.createElement('div', { className: 'content' },
      React.createElement('div', { className: 'toolbar' },
        React.createElement('input', {
          className: 'search-input',
          placeholder: 'Search by operation, service or trace ID...',
          value: search,
          onChange: e => setSearch(e.target.value),
        }),
        React.createElement('select', {
          className: 'btn',
          value: limit,
          onChange: e => setLimit(Number(e.target.value)),
        },
          [50, 100, 200, 500].map(n =>
            React.createElement('option', { key: n, value: n }, `${n} traces`)
          )
        ),
        React.createElement('button', {
          className: 'btn',
          onClick: fetchTraces,
        }, 'Refresh'),
      ),
      React.createElement('div', { className: 'trace-list' },
        filtered.length === 0
          ? React.createElement('div', { className: 'empty-state' }, 'No traces found')
          : filtered.map(t =>
              React.createElement(TraceCard, {
                key: t.trace_id,
                trace: t,
                expanded: expanded === t.trace_id,
                spans: traceSpans[t.trace_id],
                onToggle: () => toggleTrace(t.trace_id),
              })
            )
      ),
      React.createElement('div', { className: 'stats-bar' },
        React.createElement('span', { className: 'stat-item' }, `${filtered.length} traces`)
      )
    )
  );
}

function TraceCard({ trace, expanded, spans, onToggle }) {
  const statusClass = (trace.status || 'unset').toLowerCase();

  return React.createElement('div', { className: 'trace-card' },
    React.createElement('div', { className: 'trace-header', onClick: onToggle },
      React.createElement('div', { style: { display: 'flex', alignItems: 'center', gap: '10px', flex: 1, minWidth: 0 } },
        React.createElement('span', { className: `trace-status ${statusClass}` },
          statusClass === 'ok' ? '\u2713' : statusClass === 'error' ? '\u2717' : '\u25CB'
        ),
        React.createElement('span', { className: 'trace-name', style: { overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' } }, trace.operation_name),
        React.createElement('span', { className: 'trace-service' }, trace.service_name),
      ),
      React.createElement('div', { className: 'trace-meta' },
        React.createElement('span', { className: 'trace-duration' }, formatDuration(trace.start_time, trace.end_time)),
        React.createElement('span', { style: { color: 'var(--text-muted)', fontFamily: 'var(--font-mono)', fontSize: '10px' } }, trace.trace_id.slice(0, 12) + '...'),
        React.createElement('span', { style: { color: 'var(--text-muted)' } }, formatTs(trace.start_time)),
      ),
    ),
    expanded && spans ? React.createElement(SpanTimeline, { spans }) : null
  );
}

function SpanTimeline({ spans }) {
  if (!spans || spans.length === 0) return null;

  const sorted = [...spans].sort((a, b) => new Date(a.start_time) - new Date(b.start_time));
  const traceStart = new Date(sorted[0].start_time).getTime();
  const traceEnd = Math.max(...sorted.map(s => new Date(s.end_time).getTime()));
  const totalDuration = traceEnd - traceStart || 1;

  // Build tree
  const byId = {};
  sorted.forEach(s => { byId[s.span_id] = s; });

  const getDepth = (span, visited = new Set()) => {
    if (!span.parent_span_id || visited.has(span.span_id)) return 0;
    visited.add(span.span_id);
    const parent = byId[span.parent_span_id];
    return parent ? 1 + getDepth(parent, visited) : 0;
  };

  return React.createElement('div', { className: 'span-detail' },
    sorted.map(span => {
      const start = new Date(span.start_time).getTime();
      const end = new Date(span.end_time).getTime();
      const left = ((start - traceStart) / totalDuration * 100);
      const width = Math.max(((end - start) / totalDuration * 100), 0.5);
      const depth = getDepth(span);
      const statusClass = (span.status || 'unset').toLowerCase();

      return React.createElement('div', { key: span.span_id, className: 'span-row' },
        Array.from({ length: depth }).map((_, i) =>
          React.createElement('span', { key: i, className: 'span-indent' })
        ),
        React.createElement('span', { className: 'span-op', title: span.operation_name }, span.operation_name),
        React.createElement('div', { className: 'span-bar-track' },
          React.createElement('div', {
            className: `span-bar ${statusClass}`,
            style: { left: `${left}%`, width: `${width}%` },
          })
        ),
        React.createElement('span', { className: 'span-dur' }, formatDuration(span.start_time, span.end_time)),
      );
    })
  );
}

// ---- App ----

function App() {
  const [view, setView] = useState('logs');

  return React.createElement('div', { className: 'app' },
    React.createElement('div', { className: 'header' },
      React.createElement('span', { className: 'header-logo' }, 'rgrab'),
      React.createElement('div', { className: 'header-nav' },
        React.createElement('button', {
          className: view === 'logs' ? 'active' : '',
          onClick: () => setView('logs'),
        }, 'Logs'),
        React.createElement('button', {
          className: view === 'traces' ? 'active' : '',
          onClick: () => setView('traces'),
        }, 'Traces'),
      ),
    ),
    view === 'logs'
      ? React.createElement(LogsView)
      : React.createElement(TracesView)
  );
}

const root = ReactDOM.createRoot(document.getElementById('root'));
root.render(React.createElement(App));
