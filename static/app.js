const { useState, useEffect, useMemo, useCallback, useRef } = React;

function App() {
    const [flows, setFlows] = useState([]);
    const [filters, setFilters] = useState({ search: '', verdict: '' });
    const [debouncedSearch, setDebouncedSearch] = useState('');
    const [containerHeight, setContainerHeight] = useState(() => Math.max(window.innerHeight - 250, 400));
    const [scrollTop, setScrollTop] = useState(0);
    const [isPaused, setIsPaused] = useState(false);
    const [bufferedFlows, setBufferedFlows] = useState([]);
    const isPausedRef = useRef(isPaused);
    isPausedRef.current = isPaused;
    
    const scrollContainerRef = useRef(null);
    const rowHeightsRef = useRef({});
    
    const CACHE_SIZE = 100000;
    const ROW_BASE_HEIGHT = 35;
    const ROW_HEIGHT_PER_20CHARS = 10;
    const BUFFER_ROWS = 10;

    useEffect(() => {
        const timeout = setTimeout(() => {
            setDebouncedSearch(filters.search);
        }, 300);
        
        return () => clearTimeout(timeout);
    }, [filters.search]);

    useEffect(() => {
        const handleResize = () => {
            setContainerHeight(Math.max(window.innerHeight - 250, 400));
        };
        
        window.addEventListener('resize', handleResize);
        return () => window.removeEventListener('resize', handleResize);
    }, []);

    const highlightText = useCallback((text, query) => {
        if (!query || !text) return text || '';
        
        const lowerText = text.toLowerCase();
        const lowerQuery = query.toLowerCase();
        const parts = [];
        let lastIndex = 0;
        let index = lowerText.indexOf(lowerQuery);
        
        while (index !== -1) {
            if (index > lastIndex) {
                parts.push(text.substring(lastIndex, index));
            }
            parts.push(
                React.createElement('span', { 
                    key: `highlight-${index}`, 
                    className: 'search-highlight' 
                }, text.substring(index, index + query.length))
            );
            lastIndex = index + query.length;
            index = lowerText.indexOf(lowerQuery, lastIndex);
        }
        
        if (lastIndex < text.length) {
            parts.push(text.substring(lastIndex));
        }
        
        return parts.length === 1 ? parts[0] : parts;
    }, []);

    const filteredFlows = useMemo(() => {
        let result = flows;

        if (debouncedSearch) {
            const query = debouncedSearch.toLowerCase();
            result = result.filter(flow => {
                return (
                    (flow.source.namespace && flow.source.namespace.toLowerCase().includes(query)) ||
                    (flow.source.pod && flow.source.pod.toLowerCase().includes(query)) ||
                    flow.source.ip.toLowerCase().includes(query) ||
                    String(flow.source.port).includes(query) ||
                    (flow.destination.namespace && flow.destination.namespace.toLowerCase().includes(query)) ||
                    (flow.destination.pod && flow.destination.pod.toLowerCase().includes(query)) ||
                    flow.destination.ip.toLowerCase().includes(query) ||
                    String(flow.destination.port).includes(query) ||
                    flow.protocol.toLowerCase().includes(query) ||
                    flow.verdict.toLowerCase().includes(query) ||
                    flow.timestamp.toLowerCase().includes(query)
                );
            });
        }

        if (filters.verdict) {
            if (filters.verdict === 'forwarded') {
                result = result.filter(flow => flow.verdict === 'FORWARDED');
            } else if (filters.verdict === 'non-forwarded') {
                result = result.filter(flow => flow.verdict !== 'FORWARDED');
            }
        }

        return result;
    }, [flows, debouncedSearch, filters.verdict]);

    const getRowHeight = useCallback((index) => {
        if (rowHeightsRef.current[index] !== undefined) {
            return rowHeightsRef.current[index];
        }
        return ROW_BASE_HEIGHT;
    }, []);

    const setRowHeight = useCallback((index, height) => {
        if (rowHeightsRef.current[index] !== height) {
            rowHeightsRef.current[index] = height;
        }
    }, []);

    const getTotalHeight = useCallback(() => {
        let total = 0;
        for (let i = 0; i < filteredFlows.length; i++) {
            total += getRowHeight(i);
        }
        return total;
    }, [filteredFlows, getRowHeight]);

    const getVisibleRange = useCallback(() => {
        const totalHeight = getTotalHeight();
        if (totalHeight === 0) return { start: 0, end: 0 };
        
        let accumulatedHeight = 0;
        let start = 0;
        let end = filteredFlows.length;
        
        // Find start index
        for (let i = 0; i < filteredFlows.length; i++) {
            const height = getRowHeight(i);
            if (accumulatedHeight + height >= scrollTop) {
                start = Math.max(0, i - BUFFER_ROWS);
                break;
            }
            accumulatedHeight += height;
        }
        
        // Find end index
        accumulatedHeight = 0;
        for (let i = 0; i < filteredFlows.length; i++) {
            accumulatedHeight += getRowHeight(i);
            if (accumulatedHeight >= scrollTop + containerHeight) {
                end = Math.min(filteredFlows.length, i + BUFFER_ROWS);
                break;
            }
        }
        
        return { start, end };
    }, [scrollTop, containerHeight, filteredFlows, getRowHeight, getTotalHeight]);

    const getItemOffset = useCallback((index) => {
        let offset = 0;
        for (let i = 0; i < index; i++) {
            offset += getRowHeight(i);
        }
        return offset;
    }, [getRowHeight]);

    const handleScroll = useCallback((e) => {
        setScrollTop(e.target.scrollTop);
    }, []);

    const formatSourceStr = (source) => {
        const parts = [];
        if (source.namespace) parts.push(source.namespace);
        if (source.pod) {
            if (parts.length > 0) parts.push('/');
            parts.push(source.pod);
        }
        if (source.ip) {
            if (parts.length > 0) parts.push(' ');
            parts.push(`(${source.ip})`);
        }
        return parts.join('');
    };

    const formatDestStr = (dest) => {
        const parts = [];
        if (dest.namespace) parts.push(dest.namespace);
        if (dest.pod) {
            if (parts.length > 0) parts.push('/');
            parts.push(dest.pod);
        }
        if (dest.ip) {
            if (parts.length > 0) parts.push(' ');
            parts.push(`(${dest.ip})`);
        }
        return parts.join('');
    };

    const formatSource = useCallback((source, query) => {
        const combined = formatSourceStr(source);
        return query ? highlightText(combined, query) : combined;
    }, [highlightText]);

    const formatDest = useCallback((dest, query) => {
        const combined = formatDestStr(dest);
        return query ? highlightText(combined, query) : combined;
    }, [highlightText]);

    const getVerdictColor = useCallback((verdict) => {
        switch (verdict.toLowerCase()) {
            case 'forwarded': return '#28a745';
            case 'dropped': return '#dc3545';
            case 'audit': return '#ffc107';
            case 'error': return '#fd7e14';
            default: return '#ccc';
        }
    }, []);

    const clearFlows = useCallback(() => {
        setFlows([]);
        rowHeightsRef.current = {};
    }, []);

    const handleSearchChange = useCallback((e) => {
        setFilters(prev => ({ ...prev, search: e.target.value }));
    }, []);

    const handleVerdictChange = useCallback((e) => {
        setFilters(prev => ({ ...prev, verdict: e.target.value }));
    }, []);

    const togglePause = useCallback(() => {
        setIsPaused(prev => {
            const wasPaused = !prev;
            if (wasPaused) {
                // Resume - auto catch up buffered flows
                setFlows(prevFlows => {
                    const combined = [...bufferedFlows, ...prevFlows];
                    if (combined.length > CACHE_SIZE) {
                        return combined.slice(0, CACHE_SIZE);
                    }
                    return combined;
                });
                setBufferedFlows([]);
            }
            return !prev;
        });
    }, [bufferedFlows]);

    const catchUp = useCallback(() => {
        setFlows(prevFlows => {
            const combined = [...bufferedFlows, ...prevFlows];
            if (combined.length > CACHE_SIZE) {
                return combined.slice(0, CACHE_SIZE);
            }
            return combined;
        });
        setBufferedFlows([]);
    }, [bufferedFlows]);

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
                    
                    if (isPausedRef.current) {
                        setBufferedFlows(prev => {
                            const updated = [flow, ...prev];
                            if (updated.length > CACHE_SIZE) {
                                return updated.slice(0, CACHE_SIZE);
                            }
                            return updated;
                        });
                    } else {
                        setFlows(prevFlows => {
                            const updated = [flow, ...prevFlows];
                            if (updated.length > CACHE_SIZE) {
                                return updated.slice(0, CACHE_SIZE);
                            }
                            return updated;
                        });
                    }
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

    const totalFilteredFlows = filteredFlows.length;
    const hasFlows = flows.length > 0;
    const hasFilteredFlows = totalFilteredFlows > 0;
    const { start: visibleStart, end: visibleEnd } = getVisibleRange();
    const totalHeight = getTotalHeight();
    const visibleOffset = getItemOffset(visibleStart);

    const rowRefs = useRef({});

    const measureRow = useCallback((index, element) => {
        if (element) {
            const height = element.getBoundingClientRect().height;
            if (height > 0) {
                setRowHeight(index, height);
            }
        }
    }, [setRowHeight]);

    return React.createElement('div', { className: 'app' },
        React.createElement('header', null,
            React.createElement('h1', null, 'Cilium Monitor'),
            React.createElement('div', { className: 'header-controls' },
                React.createElement('div', { className: 'stats' },
                    React.createElement('span', null, `Total: ${flows.length}`),
                    React.createElement('span', null, `Filtered: ${totalFilteredFlows}`)
                ),
                React.createElement('button', {
                    className: `pause-btn ${isPaused ? 'paused' : 'playing'}`,
                    onClick: togglePause,
                    title: isPaused ? 'Resume' : 'Pause'
                }, isPaused ? '▶' : '⏸'),
                bufferedFlows.length > 0 && React.createElement('div', { className: 'buffer-control' },
                    React.createElement('span', { className: 'buffer-count' },
                        `${bufferedFlows.length} buffered`
                    ),
                    React.createElement('button', {
                        className: 'catchup-btn',
                        onClick: catchUp,
                        title: 'Catch up on buffered flows'
                    }, 'Catch up')
                )
            )
        ),
        React.createElement('div', { className: 'filters' },
            React.createElement('input', {
                type: 'text',
                placeholder: 'Search flows...',
                value: filters.search,
                onChange: handleSearchChange
            }),
            React.createElement('select', {
                value: filters.verdict,
                onChange: handleVerdictChange
            },
                React.createElement('option', { value: '' }, 'All verdicts'),
                React.createElement('option', { value: 'forwarded' }, 'Forwarded only'),
                React.createElement('option', { value: 'non-forwarded' }, 'Non-forwarded only')
            ),
            React.createElement('button', { onClick: clearFlows }, 'Clear')
        ),
        
        !hasFlows && React.createElement('div', { className: 'empty-state' },
            React.createElement('p', null, 'Waiting for traffic...')
        ),
        
        hasFlows && !hasFilteredFlows && React.createElement('div', { className: 'empty-state' },
            React.createElement('p', null, 'No flows match your search criteria')
        ),
        
        hasFlows && hasFilteredFlows && React.createElement('div', null,
            React.createElement('div', { className: 'table-header' },
                React.createElement('div', null, 'Timestamp'),
                React.createElement('div', null, 'Source (NS/Pod/IP)'),
                React.createElement('div', null, 'Dest (NS/Pod/IP)'),
                React.createElement('div', null, 'Src Port'),
                React.createElement('div', null, 'Dst Port'),
                React.createElement('div', null, 'Protocol'),
                React.createElement('div', null, 'Verdict')
            ),
            React.createElement('div', { 
                className: 'virtual-scroll-container', 
                style: { height: containerHeight },
                onScroll: handleScroll,
                ref: scrollContainerRef
            },
                React.createElement('div', { 
                    className: 'virtual-scroll-spacer',
                    style: { height: totalHeight }
                },
                    React.createElement('div', { 
                        className: 'virtual-scroll-content',
                        style: { transform: `translateY(${visibleOffset}px)` }
                    },
                        filteredFlows.slice(visibleStart, visibleEnd).map((flow, idx) => {
                            const actualIndex = visibleStart + idx;
                            return React.createElement('div', { 
                                key: flow.id,
                                className: 'flow-row',
                                ref: (el) => measureRow(actualIndex, el)
                            },
                                React.createElement('div', { className: 'flow-cell timestamp' }, highlightText(flow.timestamp, debouncedSearch)),
                                React.createElement('div', { className: 'flow-cell source' }, formatSource(flow.source, debouncedSearch)),
                                React.createElement('div', { className: 'flow-cell dest' }, formatDest(flow.destination, debouncedSearch)),
                                React.createElement('div', { className: 'flow-cell port' }, highlightText(String(flow.source.port), debouncedSearch)),
                                React.createElement('div', { className: 'flow-cell port' }, highlightText(String(flow.destination.port), debouncedSearch)),
                                React.createElement('div', { className: 'flow-cell protocol' }, highlightText(flow.protocol, debouncedSearch)),
                                React.createElement('div', { className: 'flow-cell verdict', style: { color: getVerdictColor(flow.verdict) } }, highlightText(flow.verdict, debouncedSearch))
                            );
                        })
                    )
                )
            ),
            React.createElement('div', { className: 'flow-status' },
                `Showing ${totalFilteredFlows} of ${flows.length} flows`
            )
        )
    );
}

const root = ReactDOM.createRoot(document.getElementById('root'));
root.render(React.createElement(App));
