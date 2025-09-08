import mermaid from 'https://cdn.jsdelivr.net/npm/mermaid@10.9.3/dist/mermaid.esm.min.mjs';

mermaid.initialize({ 
    maxTextSize: 10000000000,
    maxEdges: 500000,
    startOnLoad: false,
    flowchart: { htmlLabels: true, curve: 'linear' },
    securityLevel: 'loose'
});

let currentTimeline = '';
let selectedGroup = window.templateVars?.selectedGroup || '';
let chartDirection = 'TD';

let modal, modalTitle, modalContent;

let excludedEntries = [];

let currentMetromap = '';

let heatmapData = null;
let heatmapSegments = [];
let maxActivityCount = 0;

function createGroupSection(title, entries) {
    return `
        <h4>${title}</h4>
        ${entries && entries.length > 0 ? `
            <div class="group-section">
                <ul class="entry-list">
                    ${entries.map(entry => {
                        let value, type;
                        if (entry && typeof entry === 'object') {
                            value = entry.value !== undefined ? entry.value : entry;
                            type = entry.type || '';
                        } else {
                            value = entry;
                            type = '';
                        }
                        const isUrl = typeof value === 'string' && value.startsWith('http');
                        const valueHtml = isUrl ? `<span class="url-value" title="${value}">${value}</span>` : `${value}`;
                        return `<li>• ${valueHtml}${type ? ` <span class="entry-type">(${type})</span>` : ''}</li>`;
                    }).join('')}
                </ul>
            </div>
        ` : '<p class="no-entries">N/A</p>'}
    `;
}

function createDetailsTable(entries, title, tooltipText) {
    return `
        <div class="entries-section">
            <h4>${title} <span class="tooltip" title="${tooltipText}">ℹ</span></h4>
            <div class="table-container">
                <table class="entries-table">
                    <thead><tr><th>Line</th><th>Timestamp</th><th>Type</th><th>Value</th></tr></thead>
                    <tbody>
                        ${entries.map(entry => `
                            <tr>
                                <td>${entry.line || ''}</td>
                                <td>${entry.timestamp || ''}</td>
                                <td>${entry.type || ''}</td>
                                <td class="value-cell" title="${entry.value || ''}">${entry.value || ''}</td>
                            </tr>
                        `).join('')}
                    </tbody>
                </table>
            </div>
        </div>
    `;
}

window.showNodeDetails = function(nodeId) {
    const cleanNodeId = nodeId.startsWith('flowchart-') ? nodeId.substring(10) : nodeId;
    fetch(`/api/v2/node-details/${cleanNodeId}`)
        .then(response => response.json())
        .then(data => {
            if (data.error) {
                alert(`Node "${cleanNodeId}" not found`);
                return;
            }
            showNodeOrGroupModal(data);
        })
        .catch(error => alert(`Failed to fetch details: ${error.message}`));
}

function showNodeOrGroupModal(nodeData) {
    let content = '<div class="node-info">';
    if (nodeData.type === 'LineEntries') {
        content += `<h4>All entries on line ${nodeData.line}</h4>`;
        if (Array.isArray(nodeData.entries) && nodeData.entries.length > 0) {
            content += createDetailsTable(nodeData.entries, 'Line entries', 'All entries that appeared on this log line');
        } else {
            content += '<p class="no-entries">N/A</p>';
        }
    }
    else if (nodeData.type === 'GroupMerged') {
        content += '<h4>Merged groups</h4>';
        if (Array.isArray(nodeData.merged_groups)) {
            nodeData.merged_groups.forEach((group, idx) => {
                content += `<div class="group-section"><b>Group ${idx + 1}:</b><br/>`;
                if (Array.isArray(group.entries) && group.entries.length > 0) {
                    content += '<ul class="entry-list">';
                    group.entries.forEach(entry => {
                        content += `<li>• ${entry.value} <span class="entry-type">(${entry.type})</span></li>`;
                    });
                    content += '</ul>';
                } else {
                    content += '<p class="no-entries">N/A</p>';
                }
                content += '</div>';
            });
        } else {
            content += '<p>N/A</p>';
        }
        const allEntries = Array.isArray(nodeData.entries) ? nodeData.entries : [];
        const uniqueValues = [...new Set(allEntries.map(e => e.value))];
        content += createGroupSection('All unique entries after merge', uniqueValues);
        content += '<p><b>Reason:</b> Two separate groups were merged because of a tuple connection.</p><br>';
        if (Array.isArray(nodeData.merging_tuple) && nodeData.merging_tuple.length > 0) {
            content += createDetailsTable(nodeData.merging_tuple, 'Merging tuple details', 'Details of the entries that caused the merge');
        } else {
            content += '<h4>Merging tuple details <span class="tooltip" title="Details of the entries that caused the merge">ℹ</span></h4><p class="no-entries">N/A</p>';
        }
    }
    else if ((nodeData.type === 'Group' || nodeData.type === 'GroupFormed' || nodeData.type === 'GroupAdded' || nodeData.type === 'GroupJoined' || nodeData.type === 'Added') && Array.isArray(nodeData.formed_from) && nodeData.formed_from.length > 0) {
        if (nodeData.type === 'GroupFormed') {
            const tupleEntries = nodeData.formed_from.filter(e => e && e.tuple_line !== undefined);
            if (tupleEntries.length >= 2 && tupleEntries.every(e => e.tuple_line === tupleEntries[0].tuple_line)) {
                content += `<h4>Group formed from tuple:</h4><p><b>Line ${tupleEntries[0].line}</b> at time ${tupleEntries[0].timestamp}</p>`;
                content += createGroupSection('', tupleEntries).replace('<h4></h4>', '');
                content += '<p><b>Reason:</b> These entries were found together on the same line and formed a tuple. Neither entry belonged to an existing group.</p><br>';
                content += createDetailsTable(nodeData.formed_from, 'Forming tuple details', 'Details of the entries that caused this group to form');
            }
        } else if (['GroupAdded', 'GroupJoined', 'Added'].includes(nodeData.type)) {
            const uniqueValues = nodeData.entries ? [...new Set(nodeData.entries.map(entry => entry.value))] : [];
            content += createGroupSection('Group now has:', uniqueValues);
            content += '<p><b>Reason:</b> Entry was added to the group because of tuple below.</p><br>';
            content += createDetailsTable(nodeData.formed_from, 'Adding tuple details', 'Details of the entries that caused this group to form');
        }
    }
    else if (nodeData.type === 'Group' && nodeData.merge_log && nodeData.merge_log.length > 0) {
        const formedLog = nodeData.merge_log.find(e => e.startsWith('FORMED:'));
        if (formedLog) {
            const match = formedLog.match(/FORMED: \((.+?) , (.+?)\)/);
            if (match) {
                const [entryA, entryB] = [match[1].trim(), match[2].trim()];
                const entryAinfo = nodeData.entries ? nodeData.entries.find(e => e.value === entryA) : null;
                const entryBinfo = nodeData.entries ? nodeData.entries.find(e => e.value === entryB) : null;
                const formattedEntries = [
                    entryAinfo ? `${entryA} <span class="entry-type">(line ${entryAinfo.line}, time ${entryAinfo.timestamp})</span>` : entryA,
                    entryBinfo ? `${entryB} <span class="entry-type">(line ${entryBinfo.line}, time ${entryBinfo.timestamp})</span>` : entryB
                ];
                content += createGroupSection('Group formed from:', formattedEntries);
                content += '<p><b>Reason:</b> Both entries were not in any group, so a new group was formed.</p>';
                content += createDetailsTable([entryAinfo, entryBinfo].filter(Boolean), 'Forming tuple details', 'Details of the two entries that caused this group to form');
            }
        }
    }
    content += '</div>';
    showModal(nodeData.value, content);
}

function addClickEvents() {
    document.querySelectorAll('.clickable-node')
        .forEach(el => el.classList.remove('clickable-node'));
    
    document.querySelectorAll('.mermaid svg g[id]').forEach((element) => {
        if (element.id && !element.dataset.clickAdded) {
            element.classList.add('clickable-node');
            element.dataset.clickAdded = 'true';
            element.addEventListener('click', function(e) {
                e.stopPropagation();
                showNodeDetails(element.id);
            });
        }
    });
}

function addGroupClickEvents() {
    document.querySelectorAll('.mermaid svg g[id]').forEach((element) => {
        if (element.id && element.id.startsWith('G')) {
            const clean = element.cloneNode(true);
            element.parentNode.replaceChild(clean, element);
            clean.classList.add('clickable-node');
            clean.addEventListener('click', (e) => {
                e.stopPropagation();
                showNodeDetails(clean.id);
            });
        }
    });
}

function renderMermaid(timeline) {
    document.getElementById('formation-timeline-container').innerHTML = `<div class="mermaid">${timeline}</div>`;
    mermaid.init(undefined, document.querySelector('#formation-timeline-container .mermaid'));
    setTimeout(() => {
        addClickEvents();
        addGroupClickEvents();
    }, 1000);
}

async function updateTimeline() {
    try {
        const response = await fetch('/api/v2/timeline');
        const data = await response.json();
        if (data.timeline !== currentTimeline) {
            currentTimeline = data.timeline;
            selectedGroup = data.selected_group;
            renderMermaid(currentTimeline);
            const sel = document.getElementById('selected-group');
            if (sel) sel.textContent = selectedGroup || 'None';
        }
    } catch (error) {
        console.error('Error updating timeline:', error);
    }
}

function updateTimelineWithDirection() {
    if (!currentTimeline || typeof currentTimeline !== 'string' || currentTimeline.trim() === '') {
        console.error('Timeline data is empty or invalid');
        return;
    }
    const lines = currentTimeline.split('\n');
    for (let i = 0; i < lines.length; i++) {
        if (lines[i].trim().startsWith('flowchart ')) {
            lines[i] = `flowchart ${chartDirection}`;
            renderMermaid(lines.join('\n'));
            return;
        }
    }
    console.warn('Could not find flowchart definition');
}

function showModal(title, content) {
    modalTitle.textContent = title;
    modalContent.innerHTML = content;
    modal.style.display = 'block';
}

function selectGroup(groupId) {
    chartDirection = 'TD';
    document.getElementById('direction-btn').textContent = 'Top Down';
    
    document.getElementById('formation-timeline-container').innerHTML = '<div class="loading">Loading formation timeline...</div>';
    document.getElementById('nodes-container').innerHTML = '<div class="loading">Loading nodes visualization...</div>';
    document.getElementById('timelineHeatmapBar').innerHTML = '<div class="loading">Loading heatmap...</div>';
    
    fetch(`/api/v2/select-group/${groupId}`)
        .then(response => response.json())
        .then(data => {
            if (data.success) {
                selectedGroup = groupId;
                
                fetch(`/api/v2/visualization/formation/${groupId}`)
                    .then(response => response.json())
                    .then(formationData => {
                        if (formationData.success) {
                            currentTimeline = formationData.visualization;
                            renderMermaid(formationData.visualization);
                        }
                    })
                    .catch(error => console.error('Error loading formation:', error));
                
                fetch(`/api/v2/visualization/nodes/${groupId}`)
                    .then(response => response.json())
                    .then(nodesData => {
                        if (nodesData.success) {
                            const nodesContainer = document.getElementById('nodes-container');
                            nodesContainer.innerHTML = `<div class="mermaid">${nodesData.visualization}</div>`;
                            mermaid.init(undefined, nodesContainer.querySelector('.mermaid'));
                            setTimeout(() => addClickEvents(), 1000);
                        }
                    })
                    .catch(error => console.error('Error loading nodes:', error));
                
                fetch(`/api/v2/timeline-heatmap/${groupId}`)
                    .then(response => response.json())
                    .then(data => {
                        if (data.success) {
                            heatmapData = data;
                            heatmapSegments = data.segments;
                            maxActivityCount = heatmapSegments.length > 0 ? 
                                Math.max(...heatmapSegments.map(s => s.count)) : 0;
                            
                            renderTimelineHeatmap();
                            renderHeatmapStatistics();
                        }
                    })
                    .catch(error => console.error('Error loading heatmap:', error));
            } else {
                alert('Error selecting group: ' + (data.error || 'Unknown error'));
            }
        })
        .catch(error => {
            console.error('Error selecting group:', error);
            alert('Error selecting group: ' + error.message);
        });
}

async function loadGroups() {
    try {
        const response = await fetch('/api/v2/groups');
        const groups = await response.json();
        const selector = document.getElementById('group-selector');
        selector.innerHTML = '<option value="">Select a group...</option>';
        Object.keys(groups).forEach(groupId => {
            const group = groups[groupId];
            const option = document.createElement('option');
            option.value = groupId;
            option.textContent = `${groupId} (${group.count} entries, ${group.nodes} nodes)`;
            if (groupId === selectedGroup) option.selected = true;
            selector.appendChild(option);
        });
        selector.addEventListener('change', function() {
            if (this.value) selectGroup(this.value);
        });
    } catch (error) {
        console.error('Error loading groups:', error);
    }
}

function renderTimelineHeatmap() {
    if (!heatmapData || !heatmapSegments.length) return;
    
    const heatmapContainer = document.getElementById('timelineHeatmapBar');
    if (!heatmapContainer) return;
    
    const startSpan = document.getElementById('heatmapStart');
    const endSpan = document.getElementById('heatmapEnd');
    if (startSpan) startSpan.textContent = new Date(heatmapData.min_timestamp).toLocaleString();
    if (endSpan) endSpan.textContent = new Date(heatmapData.max_timestamp).toLocaleString();
    
    heatmapContainer.innerHTML = '';
    const fragment = document.createDocumentFragment();
    
    const segmentWidth = 100 / heatmapSegments.length;
    const activityColors = {
        'none': '#e0e0e0',
        'low': '#81c784',
        'medium': '#ffb74d', 
        'high': '#f06292',
        'extreme': '#e57373'
    };
    
    heatmapSegments.forEach((segment, index) => {
        const segmentDiv = document.createElement('div');
        segmentDiv.className = 'heatmap-segment';
        
        const intensity = maxActivityCount > 0 ? segment.count / maxActivityCount : 0;
        const backgroundColor = activityColors[segment.activity_level] || activityColors.none;
        
        segmentDiv.style.left = `${index * segmentWidth}%`;
        segmentDiv.style.width = `${segmentWidth}%`;
        segmentDiv.style.backgroundColor = backgroundColor;
        segmentDiv.style.opacity = Math.max(0.1, intensity);
        
        segmentDiv.title = `${segment.count} entries (${segment.activity_level})\nType: ${segment.dominant_type}\nClick to view entries`;
        
        segmentDiv.onmouseenter = () => {
            segmentDiv.style.transform = 'scaleY(1.2)';
            segmentDiv.style.zIndex = '10';
        };
        segmentDiv.onmouseleave = () => {
            segmentDiv.style.transform = 'scaleY(1)';
            segmentDiv.style.zIndex = '1';
        };
        
        segmentDiv.onclick = () => {
            showHeatmapSegmentEntries(segment.start, segment.end);
        };
        
        fragment.appendChild(segmentDiv);
    });
    
    heatmapContainer.appendChild(fragment);
    renderTimeLabels(heatmapContainer);
}

function renderTimeLabels(container) {
    if (!heatmapData?.min_timestamp || !heatmapData?.max_timestamp) {
        return;
    }
    
    const startTime = new Date(heatmapData.min_timestamp);
    const endTime = new Date(heatmapData.max_timestamp);
    const totalDuration = endTime - startTime;
    const segmentCount = heatmapSegments.length;
    
    const activityMap = new Map();
    heatmapSegments.forEach((segment, index) => {
        activityMap.set(index, {
            level: segment.activity_level,
            hasActivity: ['medium', 'high', 'extreme'].includes(segment.activity_level)
        });
    });
    
    const fragment = document.createDocumentFragment();
    let lastDateStr = null;
    
    for (let i = 0; i < 12; i++) {
        const position = (i * 100) / 11;
        const labelTime = new Date(startTime.getTime() + (totalDuration * i / 11));
        
        const labelDiv = document.createElement('div');
        labelDiv.className = 'heatmap-time-label';
        labelDiv.textContent = labelTime.toLocaleTimeString('fi-FI', { 
            hour: '2-digit', 
            minute: '2-digit',
            hour12: false
        }).replace('.', ':');
        
        labelDiv.style.left = `${position}%`;
        labelDiv.style.transform = 'translateX(-50%)';
        
        const segmentIndex = Math.floor((labelTime - startTime) / totalDuration * segmentCount);
        const activity = activityMap.get(segmentIndex);
        
        if (activity?.hasActivity) {
            const level = activity.level;
            labelDiv.classList.add(
                level === 'extreme' ? 'high-activity' :
                level === 'high' ? 'medium-activity' : 'low-activity'
            );
        } else {
            labelDiv.classList.add('no-activity');
        }
        
        fragment.appendChild(labelDiv);
        
        const currentDateStr = labelTime.toLocaleDateString('fi-FI');
        if (lastDateStr !== null && currentDateStr !== lastDateStr) {
            const dateDiv = document.createElement('div');
            dateDiv.className = 'heatmap-date-label';
            
            const dateParts = currentDateStr.split('.');
            dateDiv.innerHTML = `${dateParts[1]}-${dateParts[0]}<br>${dateParts[2]}`;
            
            dateDiv.style.left = `${position}%`;
            dateDiv.style.transform = 'translateX(-50%)';
            
            fragment.appendChild(dateDiv);
        }
        lastDateStr = currentDateStr;
    }
    
    container.appendChild(fragment);
}

function showAllGroupEntries() {
    if (!selectedGroup || !heatmapData) return;
    showModal(`All Entries (Group ${selectedGroup})`, '<div class="loading">Loading all entries...</div>');
    const startISO = encodeURIComponent(heatmapData.min_timestamp);
    const endISO = encodeURIComponent(heatmapData.max_timestamp);
    fetch(`/api/v2/heatmap-entries/${selectedGroup}?start=${startISO}&end=${endISO}`)
        .then(r => r.json())
        .then(data => {
            if (!data.success) {
                modalContent.innerHTML = '<p class="no-entries">Failed to load entries.</p>';
                return;
            }
            const entries = Array.isArray(data.entries) ? data.entries.slice() : [];
            if (!entries.length) {
                modalContent.innerHTML = '<p class="no-entries">No entries in this range.</p>';
                return;
            }
            entries.sort((a,b) => ( (a.line||0) - (b.line||0) ) || String(a.timestamp||'').localeCompare(String(b.timestamp||'')) );
            
            let content = `
                <div class="entries-section">
                    <h4>All ${entries.length} entries <span class="tooltip" title="All entries for this group in the full timeline range">ℹ</span></h4>
                    <table class="entries-table">
                        <thead><tr><th>Line</th><th>Timestamp</th><th>Type</th><th>Value</th></tr></thead>
                        <tbody>`;
                        
            entries.forEach(entry => {
                content += `<tr>
                    <td>${entry.line || ''}</td>
                    <td>${entry.timestamp || ''}</td>
                    <td>${entry.type || ''}</td>
                    <td class="value-cell" title="${entry.value || ''}">${entry.value || ''}</td>
                </tr>`;
            });
                        
            content += `</tbody>
                    </table>
                </div>`;
            
            modalContent.innerHTML = content;
        })
        .catch(e => {
            modalContent.innerHTML = `<p class=\"no-entries\">Error: ${e.message}</p>`;
        });
}

function showAllUniqueEntries() {
    if (!selectedGroup || !heatmapData) return;
    showModal(`Unique Entries (Group ${selectedGroup})`, '<div class="loading">Loading unique entries...</div>');
    const startISO = encodeURIComponent(heatmapData.min_timestamp);
    const endISO = encodeURIComponent(heatmapData.max_timestamp);
    fetch(`/api/v2/heatmap-entries/${selectedGroup}?start=${startISO}&end=${endISO}`)
        .then(r => r.json())
        .then(data => {
            if (!data.success) {
                modalContent.innerHTML = '<p class="no-entries">Failed to load entries.</p>';
                return;
            }
            const entries = Array.isArray(data.entries) ? data.entries.slice() : [];
            if (!entries.length) {
                modalContent.innerHTML = '<p class="no-entries">No entries in this range.</p>';
                return;
            }
            
            const uniqueMap = new Map();
            entries.forEach(entry => {
                const value = entry.value || '';
                const type = entry.type || '';
                if (value && !uniqueMap.has(value)) {
                    uniqueMap.set(value, type);
                }
            });
            
            const uniqueEntries = Array.from(uniqueMap.entries()).sort((a, b) => a[0].localeCompare(b[0]));
            
            let content = `
                <div class="entries-section">
                    <h4>${uniqueEntries.length} unique entries <span class="tooltip" title="Unique entry values for this group in the full timeline range">ℹ</span></h4>
                    <table class="entries-table unique-entries-table">
                        <thead><tr><th>Type</th><th>Value</th></tr></thead>
                        <tbody>`;
                        
            uniqueEntries.forEach(([value, type]) => {
                content += `<tr>
                    <td>${type}</td>
                    <td class="value-cell" title="${value}">${value}</td>
                </tr>`;
            });
                        
            content += `</tbody>
                    </table>
                </div>`;
            
            modalContent.innerHTML = content;
        })
        .catch(e => {
            modalContent.innerHTML = `<p class=\"no-entries\">Error: ${e.message}</p>`;
        });
}

function renderHeatmapStatistics() {
    if (!heatmapData?.statistics) {
        return;
    }
    
    const statsContainer = document.getElementById('heatmapStats');
    if (!statsContainer) {
        return;
    }
    
    const stats = heatmapData.statistics;
    const totalEntries = heatmapData.total_entries;
    
    const statCards = [
        { value: totalEntries.toLocaleString(), label: 'Total Entries', clickable: true, type: 'total' },
        { value: heatmapData.duration_days, label: 'Days Span' },
        { value: stats.avg_hourly, label: 'Avg/Hour' },
        { value: stats.unique_entries || 0, label: 'Unique Entries', clickable: true, type: 'unique' }
    ].map(card =>
        `<div class="stat-card" ${card.clickable ? `data-${card.type}-entries="1" title="Click to view ${card.type === 'total' ? 'all' : 'unique'} entries" style="cursor:pointer;"` : ''}>
            <div class="stat-value">${card.value}</div>
            <div class="stat-label">${card.label}</div>
        </div>`
    ).join('');
    
    const busiestHours = stats.busiest_hours.map(hour => 
        `<li><strong>${hour.formatted}</strong>: ${hour.count} entries</li>`
    ).join('');
    
    const topTypes = stats.top_types.map(type => 
        `<li><strong>${type.type}</strong>: ${type.count} (${type.percentage}%)</li>`
    ).join('');
    
    statsContainer.innerHTML = `
        <div class="heatmap-stat-grid">${statCards}</div>
        <div class="heatmap-details">
            <div class="detail-section">
                <h4>Busiest Hours</h4>
                <ul class="busiest-list">${busiestHours}</ul>
            </div>
            <div class="detail-section">
                <h4>Top Entry Values</h4>
                <ul class="types-list">${topTypes}</ul>
            </div>
        </div>
    `;
    const totalCard = statsContainer.querySelector('[data-total-entries]');
    if (totalCard) totalCard.addEventListener('click', showAllGroupEntries);
    
    const uniqueCard = statsContainer.querySelector('[data-unique-entries]');
    if (uniqueCard) uniqueCard.addEventListener('click', showAllUniqueEntries);
}

function showFilteredModal(filteredData) {
    modalTitle.textContent = 'Filtered Entries';

    const storageKey = 'mermetro_excludedEntries_v1';
    let savedExcluded = null;
    try {
        const raw = localStorage.getItem(storageKey);
        if (raw) savedExcluded = JSON.parse(raw);
    } catch (e) {
        console.warn('Failed to read saved excluded entries', e);
    }

    const nodeListHtml = filteredData.length > 0 ? 
        filteredData.map(value => 
            `<div class="checkbox-item">
                <input type="checkbox" id="entry_${value.replace(/[^a-zA-Z0-9]/g, '_')}" 
                    class="entry-checkbox" value="${value}">
                <label for="entry_${value.replace(/[^a-zA-Z0-9]/g, '_')}">${value}</label>
            </div>`
        ).join('') : 'N/A';

    modalContent.innerHTML = `
        <div class="detail-section">
            <p><strong>Total unique values:</strong> ${filteredData.length}</p>

            <div style="margin:10px 0;">
                <input id="newCommonValue" type="text" placeholder="Remove / Add value to common_values.txt" 
                       style="padding:6px; width:70%; margin-right:8px;">
                <button id="addCommonButton" class="filter-button">Remove / Add common value</button>
            </div>

            <p><strong>Filtered Entries:</strong>
                <span class="info-tooltip">
                    ?
                    <span class="tooltip-text">These values are either marked as filtered types or found in common_values.txt file. Checking entries will include them in the metromap logic (they can form or merge groups).</span>
                </span>    
            </p>
            <div class="checkbox-container">
                <button id="selectAllButton" class="filter-button">Select All</button>
                <button id="deselectAllButton" class="filter-button">Deselect All</button>
                <div class="checkbox-list">${nodeListHtml}</div>
            </div>
            <div class="reload-container">
                <button id="reloadMapButton" class="reload-button">Process</button>
            </div>
        </div>
    `;

    const checkboxList = modalContent.querySelector('.checkbox-list');

    checkboxList.onchange = function(e) {
        const cb = e.target;
        if (!cb || !cb.classList || !cb.classList.contains('entry-checkbox')) return;
        if (cb.checked) {
            excludedEntries = excludedEntries.filter(item => item !== cb.value);
        } else {
            if (!excludedEntries.includes(cb.value)) excludedEntries.push(cb.value);
        }
        try { localStorage.setItem(storageKey, JSON.stringify(excludedEntries)); } catch (err) {}
    };

    const selectAllBtn = document.getElementById('selectAllButton');
    const deselectAllBtn = document.getElementById('deselectAllButton');
    selectAllBtn.onclick = function() {
        checkboxList.querySelectorAll('.entry-checkbox').forEach(checkbox => checkbox.checked = true);
        excludedEntries = [];
        try { localStorage.setItem(storageKey, JSON.stringify(excludedEntries)); } catch (err) {}
    };
    deselectAllBtn.onclick = function() {
        checkboxList.querySelectorAll('.entry-checkbox').forEach(checkbox => checkbox.checked = false);
        excludedEntries = Array.from(checkboxList.querySelectorAll('.entry-checkbox')).map(cb => cb.value);
        try { localStorage.setItem(storageKey, JSON.stringify(excludedEntries)); } catch (err) {}
    };

    if (Array.isArray(savedExcluded)) {
        checkboxList.querySelectorAll('.entry-checkbox').forEach(cb => {
            cb.checked = !savedExcluded.includes(cb.value);
        });
        excludedEntries = savedExcluded.slice();
    } else {
        deselectAllBtn.click();
    }

    function performReload() {
        const normalBtn = document.getElementById('reloadMapButton');
        const activeBtn = normalBtn;
        const originalText = activeBtn.textContent;
        activeBtn.textContent = 'Processing...';
        activeBtn.disabled = true;

        fetch('/api/v2/reload', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', 'Accept': 'application/json' },
            body: JSON.stringify({ 
                excludedEntries: excludedEntries
            }),
            cache: 'no-store'
        })
        .then(response => response.json())
        .then(data => {
            if (data.success) {
                return updateMetromap().then(() => {
                    showRefreshNotification();
                });
            }
            alert('Process failed');
        })
        .finally(() => {
            activeBtn.textContent = originalText;
            activeBtn.disabled = false;
        });
    }

    document.getElementById('reloadMapButton').onclick = function() { performReload(); };

    const addCommonBtn = document.getElementById('addCommonButton');
    addCommonBtn.onclick = async function() {
        const val = document.getElementById('newCommonValue').value.trim();
        if (!val) { alert('Enter a non-empty value.'); return; }
        if (val.length > 500 || val.includes('\n') || val.includes('\r')) { alert('Value too long or contains invalid characters.'); return; }

        let originalText = addCommonBtn.textContent;
        try {
            const existing = await fetch('/api/v2/filtered-entries', { cache: 'no-store' }).then(r => r.json());
            const willRemove = Array.isArray(existing) && existing.includes(val);

            addCommonBtn.textContent = willRemove ? 'Removing...' : 'Adding...';
            addCommonBtn.disabled = true;

            const resp = await fetch('/api/v2/common/add', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json', 'Accept': 'application/json' },
                body: JSON.stringify({ value: val })
            }).then(r => r.json());

            if (resp.success) {
                if (resp.action === 'removed') alert('Removed from common_values.txt');
                else if (resp.action === 'added') alert('Added to common_values.txt');
                else alert('Updated common_values.txt');

                const data = await fetch('/api/v2/filtered-entries', { cache: 'no-store' }).then(r => r.json());
                checkboxList.innerHTML = data.map(value => 
                    `<div class="checkbox-item">
                        <input type="checkbox" id="entry_${value.replace(/[^a-zA-Z0-9]/g, '_')}" 
                            class="entry-checkbox" value="${value}">
                        <label for="entry_${value.replace(/[^a-zA-Z0-9]/g, '_')}">${value}</label>
                    </div>`
                ).join('');
                
                try {
                    const raw = localStorage.getItem(storageKey);
                    if (raw) {
                        const currentSaved = JSON.parse(raw);
                        checkboxList.querySelectorAll('.entry-checkbox').forEach(cb => {
                            cb.checked = !currentSaved.includes(cb.value);
                        });
                        excludedEntries = currentSaved.slice();
                    } else {
                        deselectAllBtn.click();
                    }
                } catch (e) {
                    deselectAllBtn.click();
                }
            } else {
                alert('Operation failed: ' + (resp.message || 'unknown'));
            }
        } catch (err) {
            console.error(err);
            alert('Request failed');
        } finally {
            addCommonBtn.textContent = originalText;
            addCommonBtn.disabled = false;
        }
    };

    modal.style.display = 'block';
}

async function updateMetromap(ignoreTimeFilters = false) {
    try {
        const params = new URLSearchParams();
        params.append('t', Date.now());

        if (ignoreTimeFilters) {
            params.append('reset', '1');
        } else {
            const startDate = document.getElementById('startDate')?.value;
            const startTime = document.getElementById('startTime')?.value;
            const endDate = document.getElementById('endDate')?.value;
            const endTime = document.getElementById('endTime')?.value;

            if (startDate) {
                const startDateTime = startTime ? `${startDate}T${startTime}` : `${startDate}T00:00:00`;
                params.append('start', startDateTime);
            }
            if (endDate) {
                const endDateTime = endTime ? `${endDate}T${endTime}` : `${endDate}T23:59:59`;
                params.append('end', endDateTime);
            }
        }

        const response = await fetch(`/api/v2/timestampfilter?${params.toString()}`);
        const data = await response.json();

        if (data.metromap !== currentMetromap) {
            currentMetromap = data.metromap;
            const container = document.getElementById('metromap-container');
            if (container) {
                container.innerHTML = `<div class="mermaid">${currentMetromap}</div>`;
                await mermaid.run({ querySelector: '#metromap-container .mermaid' });
                setTimeout(addClickEvents, 100);
            }
            const ts = document.getElementById('timestamp');
            if (ts) ts.textContent = data.timestamp;
        }
    } catch (error) {
        console.error('Update error:', error);
    }
}

function showRefreshNotification() {
    const notification = document.createElement('div');
    notification.id = 'refreshNotification';
    notification.className = 'refresh-notification';
    notification.innerHTML = `
        <div class="refresh-notification-content">
            <h3>Metromap Updated Successfully!</h3>
            <strong>Please refresh the page to see the updated groups and timeline.</strong></p>
            <div class="refresh-buttons">
                <button id="refreshPageButton" class="refresh-page-button">Refresh Page Now</button>
                <button id="dismissButton" class="dismiss-button">Dismiss</button>
            </div>
        </div>
    `;
    
    document.body.appendChild(notification);
    
    document.getElementById('refreshPageButton').addEventListener('click', function() {
        location.reload();
    });
    
    document.getElementById('dismissButton').addEventListener('click', function() {
        notification.remove();
    });
}

document.addEventListener('DOMContentLoaded', function() {
    modal = document.getElementById('nodeModal');
    modalTitle = document.getElementById('modalTitle');
    modalContent = document.getElementById('modalContent');
    
    const directionBtn = document.getElementById('direction-btn');
    if (directionBtn) {
        directionBtn.addEventListener('click', function() {
            chartDirection = (chartDirection === 'TD') ? 'LR' : 'TD';
            directionBtn.textContent = (chartDirection === 'TD') ? 'Top Down' : 'Left Right';
            updateTimelineWithDirection();
        });
    }

    const groupLogModal = document.getElementById('groupLogModal');
    
    document.querySelector('#nodeModal .close')?.addEventListener('click', () => modal.style.display = 'none');
    document.querySelector('#groupLogModal .close')?.addEventListener('click', () => groupLogModal.style.display = 'none');
    
    window.addEventListener('click', function(event) {
        if (event.target === modal) modal.style.display = 'none';
        if (event.target === groupLogModal) groupLogModal.style.display = 'none';
    });

    document.getElementById('filteredButton')?.addEventListener('click', function() {
        fetch('/api/v2/filtered-entries')
            .then(response => response.json())
            .then(data => {
                if (!data.error) {
                    showFilteredModal(data);
                }
            })
            .catch(error => console.error('Error:', error));
    });

    const applyBtn = document.getElementById('applyTimeRangeButton');
    const clearBtn = document.getElementById('clearTimeRangeButton');
    if (applyBtn) {
        applyBtn.addEventListener('click', function() {
            const startDate = document.getElementById('startDate').value;
            const startTime = document.getElementById('startTime').value;
            const endDate = document.getElementById('endDate').value;
            const endTime = document.getElementById('endTime').value;

            if (startDate && endDate) {
                const startDateTime = `${startDate}T${startTime || '00:00:00'}`;
                const endDateTime = `${endDate}T${endTime || '23:59:59'}`;

                if (startDateTime > endDateTime) {
                    alert('Start time must be before end time');
                    return;
                }
            }

            applyBtn.textContent = 'Reloading...';
            applyBtn.disabled = true;

            updateMetromap().finally(() => {
                applyBtn.textContent = 'Apply Time Range';
                applyBtn.disabled = false;
                showRefreshNotification();
            });
        });
    }
    if (clearBtn) {
        clearBtn.addEventListener('click', function() {
            clearBtn.textContent = 'Resetting...';
            clearBtn.disabled = true;

            document.getElementById('startDate').value = '';
            document.getElementById('startTime').value = '00:00:00';
            document.getElementById('endDate').value = '';
            document.getElementById('endTime').value = '23:59:59';

            updateMetromap(true).finally(() => {
                clearBtn.textContent = 'Reset';
                clearBtn.disabled = false;
                showRefreshNotification();
            });
        });
    }

    loadGroups();
    if (selectedGroup) updateTimeline();

    const observer = new MutationObserver(function(mutations) {
        mutations.forEach(function(mutation) {
            if (mutation.addedNodes.length && document.querySelector('.mermaid svg')) {
                addClickEvents();
                addGroupClickEvents();
                observer.disconnect();
            }
        });
    });
    
    const mermaidContainer = document.querySelector('.mermaid');
    if (mermaidContainer) {
        observer.observe(mermaidContainer, { childList: true, subtree: true });
    }

    const startDateEl = document.getElementById('startDate');
    const endDateEl = document.getElementById('endDate');
    if (!(startDateEl && startDateEl.value) && !(endDateEl && endDateEl.value)) {
        updateMetromap();
    }
 });

async function showHeatmapSegmentEntries(startTime, endTime) {
    try {
        showModal("Loading entries...", "<div style='text-align:center;padding:20px;'>Loading entries for selected time range...</div>");
        
        const groupParam = selectedGroup ? selectedGroup : 'all';
        const url = `/api/v2/heatmap-entries/${groupParam}?start=${startTime}&end=${endTime}`;
        
        const response = await fetch(url);
        const data = await response.json();
        
        if (!data.success) {
            throw new Error(data.error || 'Failed to load entries');
        }
        
        const startDisplay = new Date(startTime).toLocaleString();
        const endDisplay = new Date(endTime).toLocaleString();
        const title = `${data.count} Entries (${startDisplay} - ${endDisplay})`;
        
        let content = `
            <p><strong>Activity Level:</strong> ${data.count > 75 ? 'High' : (data.count > 25 ? 'Medium' : 'Low')}</p>
            <p><strong>Group:</strong> ${selectedGroup || 'All entries'}</p>
            
            <div class="entries-section">
                <h4>Time Range Entries <span class="tooltip" title="All log entries that occurred during this time period">ℹ</span></h4>
                <table class="entries-table">
                    <thead><tr><th>Line</th><th>Timestamp</th><th>Type</th><th>Value</th></tr></thead>
                    <tbody>`;
                    
        data.entries.forEach(entry => {
            content += `<tr>
                <td>${entry.line || 'N/A'}</td>
                <td>${entry.timestamp || 'N/A'}</td>
                <td>${entry.type || 'N/A'}</td>
                <td class="value-cell" title="${entry.value || ''}">${entry.value || 'N/A'}</td>
            </tr>`;
        });
                    
        content += `</tbody>
                </table>
            </div>`;
        
        showModal(title, content);
        
    } catch (error) {
        console.error('Error loading heatmap segment entries:', error);
        showModal('Error', `<p>Failed to load entries: ${error.message}</p>`);
    }
}
