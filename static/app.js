const { useState, useEffect, useMemo, useCallback } = React;

function App() {
    const [flows, setFlows] = useState([]);
    const [filters, setFilters] = useState({
        namespace: '',
        verdict: ''
    });
    const maxFlows = 200;

    const filteredFlows = useMemo(() => {
        let result = flows;

        if (filters.namespace) {
            const ns = filters.namespace.toLowerCase();
            result = result.filter(flow =>
                flow.source.namespace.toLowerCase().includes(ns) ||
                flow.destination.namespace.toLowerCase().includes(ns)
            );
        }

        if (filters.verdict) {
            result = result.filter(flow => flow.verdict === filters.verdict);
        }

        return result.slice(0, maxFlows);
    }, [flows, filters]);

    const formatSource = useCallback((source) => {
        const ns = source.namespace || '-';
        const pod = source.pod || '-';
        const ip = source.ip || '-';
        return `${ns}/${pod} (${ip})`;
    }, []);

    const formatDest = useCallback((dest) => {
        const ns = dest.namespace || '-';
        const pod = dest.pod || '-';
        const ip = dest.ip || '-';
        return `${ns}/${pod} (${ip})`;
    }, []);

    const getVerdictClass = useCallback((verdict) => verdict.toLowerCase(), []);

    const clearFlows = useCallback(() => {
        setFlows([]);
    }, []);

    const handleNamespaceChange = useCallback((e) => {
        setFilters(prev => ({ ...prev, namespace: e.target.value }));
    }, []);

    const handleVerdictChange = useCallback((e) => {
        setFilters(prev => ({ ...prev, verdict: e.target.value }));
    }, []);

    useEffect(() => {
        let ws = null;
        let reconnectTimeout = null;

        const connect = () => {
            const wsUrl = `ws://${window.location.host}/ws`;
            ws = new WebSocket(wsUrl);

            ws.onopen = () => {
                console.log('Connected to WebSocket');
            };

            ws.onmessage = (event) => {
                try {
                    const flow = JSON.parse(event.data);
                    setFlows(prevFlows => {
                        const updated = [flow, ...prevFlows];
                        return updated.length > maxFlows ? updated.slice(0, maxFlows) : updated;
                    });
                } catch (e) {
                    console.error('Error parsing flow:', e);
                }
            };

            ws.onerror = (error) => {
                console.error('WebSocket error:', error);
            };

            ws.onclose = () => {
                console.log('WebSocket closed, reconnecting...');
                reconnectTimeout = setTimeout(connect, 1000);
            };
        };

        connect();

        return () => {
            if (ws) {
                ws.close();
            }
            if (reconnectTimeout) {
                clearTimeout(reconnectTimeout);
            }
        };
    }, []);

    return React.createElement('div', { className: 'app' },
        React.createElement('header', null,
            React.createElement('h1', null, 'Cilium Monitor'),
            React.createElement('div', { className: 'stats' },
                React.createElement('span', null, `Total: ${flows.length}`),
                React.createElement('span', null, `Filtered: ${filteredFlows.length}`)
            )
        ),
        React.createElement('div', { className: 'filters' },
            React.createElement('input', {
                type: 'text',
                placeholder: 'Filter by namespace...',
                value: filters.namespace,
                onChange: handleNamespaceChange
            }),
            React.createElement('select', {
                value: filters.verdict,
                onChange: handleVerdictChange
            },
                React.createElement('option', { value: '' }, 'All verdicts'),
                React.createElement('option', null, 'FORWARDED'),
                React.createElement('option', null, 'DROPPED'),
                React.createElement('option', null, 'AUDIT'),
                React.createElement('option', null, 'ERROR')
            ),
            React.createElement('button', { onClick: clearFlows }, 'Clear')
        ),
        React.createElement('div', { className: 'table-container' },
            React.createElement('table', null,
                React.createElement('thead', null,
                    React.createElement('tr', null,
                        React.createElement('th', null, 'Timestamp'),
                        React.createElement('th', null, 'Source (NS/Pod/IP)'),
                        React.createElement('th', null, 'Dest (NS/Pod/IP)'),
                        React.createElement('th', null, 'Src Port'),
                        React.createElement('th', null, 'Dst Port'),
                        React.createElement('th', null, 'Protocol'),
                        React.createElement('th', null, 'Verdict')
                    )
                ),
                React.createElement('tbody', null,
                    filteredFlows.map(flow =>
                        React.createElement('tr', { key: flow.id },
                            React.createElement('td', null, flow.timestamp),
                            React.createElement('td', null, formatSource(flow.source)),
                            React.createElement('td', null, formatDest(flow.destination)),
                            React.createElement('td', null, flow.source.port),
                            React.createElement('td', null, flow.destination.port),
                            React.createElement('td', null, flow.protocol),
                            React.createElement('td', { className: getVerdictClass(flow.verdict) }, flow.verdict)
                        )
                    )
                )
            )
        ),
        flows.length === 0 && React.createElement('div', { className: 'empty-state' },
            React.createElement('p', null, 'No flows yet. Waiting for traffic...')
        )
    );
}

const root = ReactDOM.createRoot(document.getElementById('root'));
root.render(React.createElement(App));
