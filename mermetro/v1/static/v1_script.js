import mermaid from 'https://cdn.jsdelivr.net/npm/mermaid@10.9.3/dist/mermaid.esm.min.mjs';

mermaid.initialize({ 
    startOnLoad: false,
    flowchart: { useMaxWidth: true, htmlLabels: true },
    securityLevel: 'loose',
    maxTextSize: 10000000000,
    maxEdges: 500000
});

let currentMetromap = '';

let modal, modalTitle, modalContent, searchResults;

let filteredEntries = [];

window.showNodeDetails = function(nodeId) {
    window.currentId = nodeId;
    
    const cleanId = nodeId.startsWith('flowchart-') ? 
        nodeId.substring(10) : nodeId;
    
    window.currentGroupId = cleanId;
    
    fetch(`/api/v1/node-details/${cleanId}`)
        .then(response => response.json())
        .then(data => {
            if (!data.error) {
                showModal(data);
            }
        })
        .catch(error => console.error('Error:', error));
};

function showModal(nodeData) {
    if (nodeData.type === 'Group') {
        showGroupModal(nodeData);
    } else {
        showNodeModal(nodeData);
    }
    modal.style.display = 'block';
}

function showGroupModal(nodeData) {
    const groupNumber = window.currentGroupId?.startsWith('ID_') ? 
        window.currentGroupId.split('-')[0].split('_')[1] : '?';
    
    modalTitle.textContent = `ID: ${groupNumber}`;
    
    const personalCount = nodeData.entries?.length || 0;
    const filteredCount = nodeData.filtered_entries?.length || 0;
    const totalCount = personalCount + filteredCount;

    const uniqueValues = nodeData.entries ? 
        [...new Set(nodeData.entries.map(entry => entry.value))] : [];
    
    const nodeListHtml = uniqueValues.length > 0 ? 
        uniqueValues.map(value => `• ${value}`).join('<br/>') : 'N/A';
    
    let content = `
        <div class="detail-section">
            <p><strong>Entries:</strong> ${nodeData.value}</p>
            <p><strong>Every group unique entry:</strong><br/>${nodeListHtml}</p>
            <p><strong>Count:</strong> ${totalCount}</p>
            <p><strong>First:</strong> ${nodeData.first_seen}</p>
            <p><strong>Last:</strong> ${nodeData.last_seen}</p>
        </div>
    `;
    
    if (nodeData.merge_log?.length > 0) {
        content += `
            <div class="detail-section">
                <h4>Group formation:
                    <span class="info-tooltip">
                        ?
                        <span class="tooltip-text">
                            FORMED: Neither value of a tuple is in a group, so a new group is created.<br/>
                            JOINED: First value of a tuple is in a group, so the second value is added to that group.<br/>
                            MERGED: Both values of a tuple are in different groups, so the groups are merged.<br/>
                        </span>
                    </span>
                </h4>
                <p class="merge-log">${nodeData.merge_log.join('<br/>')}</p>
            </div>
        `;
    }
    
    content += createEntriesTable(nodeData.entries, 'Every unique entry:', 'When has this exact entry appeared in the log file?');

    if (nodeData.filtered_entries?.length > 0) {
        content += createLoadFilteredButton();
    }
    
    modalContent.innerHTML = content;
    
    if (nodeData.filtered_entries?.length > 0) {
        const loadBtn = document.getElementById('loadFilteredButton');
        if (loadBtn) {
            loadBtn.onclick = function() {
                const buttonContainer = document.getElementById('filteredButtonContainer');
                if (buttonContainer) {
                    buttonContainer.innerHTML = createFilteredTable(nodeData.filtered_entries);
                }
            };
        }
    }
}

function showNodeModal(nodeData) {
    modalTitle.textContent = `${nodeData.type}: ${nodeData.value}`;
    
    const personalCount = nodeData.entries?.length || 0;
    const filteredCount = nodeData.filtered_entries?.length || 0;
    const totalCount = personalCount + filteredCount;

    let content = `
        <div class="detail-section">
            <p><strong>Count:</strong> ${totalCount}</p>
            <p><strong>First:</strong> ${nodeData.first_seen}</p>
            <p><strong>Last:</strong> ${nodeData.last_seen}</p>
        </div>
    `;
    
    content += createEntriesTable(nodeData.entries, 'Unique entries:', 'When has this exact entry appeared in the log file?');

    if (nodeData.filtered_entries?.length > 0) {
        content += createLoadFilteredButton();
    }
    
    modalContent.innerHTML = content;
    
        if (nodeData.filtered_entries?.length > 0) {
        const loadBtn = document.getElementById('loadFilteredButton');
        if (loadBtn) {
            loadBtn.onclick = function() {
                const buttonContainer = document.getElementById('filteredButtonContainer');
                if (buttonContainer) {
                    buttonContainer.innerHTML = createFilteredTable(nodeData.filtered_entries);
                }
            };
        }
    }
}

function createEntriesTable(entries, title, tooltipText) {
    return `
        <div class="detail-section">
            <h4>${title}
                <span class="info-tooltip">
                    ?
                    <span class="tooltip-text">${tooltipText}</span>
                </span>
            </h4>
            <table class="entries-table">
                <thead>
                    <tr><th>#</th><th>Line</th><th>Time</th><th>Type</th><th>Value</th></tr>
                </thead>
                <tbody>
                    ${entries.map((entry, index) => `
                        <tr>
                            <td><strong>${index + 1}</strong></td>
                            <td><strong>${entry.line}</strong></td>
                            <td>${entry.timestamp}</td>
                            <td>${entry.type}</td>
                            <td>${entry.value}</td>
                        </tr>
                    `).join('')}
                </tbody>
            </table>
        </div>
    `;
}

function createFilteredTable(filteredEntries) {
    return `
        <div class="detail-section filtered-section">
            <p><strong>Filtered entries</strong>
                <span class="info-tooltip">
                    ?
                    <span class="tooltip-text">These entries appeared with the entry above, but have been filtered out to keep metromap readable</span>
                </span>
            </p>
            <table class="entries-table filtered-table">
                <thead>
                    <tr><th>#</th><th>Line</th><th>Time</th><th>Type</th><th>Value</th></tr>
                </thead>
                <tbody>
                    ${filteredEntries.map((entry, index) => `
                        <tr>
                            <td><strong>${index + 1}</strong></td>
                            <td><strong>${entry.line}</strong></td>
                            <td>${entry.timestamp}</td>
                            <td>${entry.type}</td>
                            <td>${entry.value}</td>
                        </tr>
                    `).join('')}
                </tbody>
            </table>
        </div>
    `;
}

function createLoadFilteredButton() {
    return `
        <div id="filteredButtonContainer" class="detail-section">
            <button id="loadFilteredButton" class="filter-button" style="background-color: #FF9800; padding: 10px 20px; font-size: 14px;">
                Load filtered entries
            </button>
        </div>
    `;
}

function showFilteredModal(filteredData) {
    modalTitle.textContent = 'Filtered Entries';

    const storageKey = 'mermetro_filteredEntries_v1';
    let savedFiltered = null;
    try {
        const raw = localStorage.getItem(storageKey);
        if (raw) savedFiltered = JSON.parse(raw);
    } catch (e) {
        console.warn('Failed to read saved filtered entries', e);
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
            filteredEntries = filteredEntries.filter(item => item !== cb.value);
        } else {
            if (!filteredEntries.includes(cb.value)) filteredEntries.push(cb.value);
        }
        try { localStorage.setItem(storageKey, JSON.stringify(filteredEntries)); } catch (err) {}
    };

    const selectAllBtn = document.getElementById('selectAllButton');
    const deselectAllBtn = document.getElementById('deselectAllButton');
    selectAllBtn.onclick = function() {
        checkboxList.querySelectorAll('.entry-checkbox').forEach(checkbox => checkbox.checked = true);
        filteredEntries = [];
        try { localStorage.setItem(storageKey, JSON.stringify(filteredEntries)); } catch (err) {}
    };
    deselectAllBtn.onclick = function() {
        checkboxList.querySelectorAll('.entry-checkbox').forEach(checkbox => checkbox.checked = false);
        filteredEntries = Array.from(checkboxList.querySelectorAll('.entry-checkbox')).map(cb => cb.value);
        try { localStorage.setItem(storageKey, JSON.stringify(filteredEntries)); } catch (err) {}
    };

    if (Array.isArray(savedFiltered)) {
        checkboxList.querySelectorAll('.entry-checkbox').forEach(cb => {
            cb.checked = !savedFiltered.includes(cb.value);
        });
        filteredEntries = savedFiltered.slice();
    } else {
        deselectAllBtn.click();
    }

    function performReload() {
        const normalBtn = document.getElementById('reloadMapButton');
        const activeBtn = normalBtn;

        const originalText = activeBtn.textContent;
        activeBtn.textContent = 'Processing...';
        normalBtn.disabled = true;

        fetch('/api/v1/reload-metromap', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', 'Accept': 'application/json' },
            body: JSON.stringify({ 
                filteredEntries: filteredEntries
            }),
            cache: 'no-store'
        })
        .then(response => response.json())
        .then(data => {
            if (data.success) {
                return updateMetromap();
            }
            alert('Process failed');
        })
        .finally(() => {
            activeBtn.textContent = originalText;
            normalBtn.disabled = false;
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
            const existing = await fetch('/api/v1/filtered-entries', { cache: 'no-store' }).then(r => r.json());
            const willRemove = Array.isArray(existing) && existing.includes(val);

            addCommonBtn.textContent = willRemove ? 'Removing...' : 'Adding...';
            addCommonBtn.disabled = true;

            const resp = await fetch('/api/v1/common/add', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json', 'Accept': 'application/json' },
                body: JSON.stringify({ value: val })
            }).then(r => r.json());

            if (resp.success) {
                if (resp.action === 'removed') alert('Removed from common_values.txt');
                else if (resp.action === 'added') alert('Added to common_values.txt');
                else alert('Updated common_values.txt');

                const data = await fetch('/api/v1/filtered-entries', { cache: 'no-store' }).then(r => r.json());
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
                        filteredEntries = currentSaved.slice();
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

        const response = await fetch(`/api/v1/metromap?${params.toString()}`);
        const data = await response.json();

        if (data.metromap !== currentMetromap) {
            currentMetromap = data.metromap;
            const container = document.getElementById('metromap-container');
            container.innerHTML = `<div class="mermaid">${currentMetromap}</div>`;

            await mermaid.run({ querySelector: '.mermaid' });
            setTimeout(addClickEvents, 100);
            const timestampEl = document.getElementById('timestamp');
            if (timestampEl) timestampEl.textContent = data.timestamp;
        }
    } catch (error) {
        console.error('Update error:', error);
    }
}

function performSearch() {
    const searchTerm = document.getElementById('searchInput').value.trim();
    if (!searchTerm) {
        searchResults.innerHTML = '';
        searchResults.style.display = 'none';
        return;
    }
    searchResults.style.display = 'block';
    searchResults.innerHTML = '<div class="search-result">Searching...</div>';
    
    fetch(`/api/v1/search/${encodeURIComponent(searchTerm)}`)
        .then(response => response.json())
        .then(data => {
            if (data.results?.length > 0) {
                if (data.results.length === 1) {
                    const result = data.results[0];
                    showNodeDetails(result.node_id);
                    searchResults.innerHTML = `<div class="search-result">Found: ${result.display_text}</div>`;
                } else {
                    searchResults.innerHTML = `
                        <div class="search-multiple">Found ${data.results.length} matches:</div>
                        ${data.results.map(result => 
                            `<div class="search-result" onclick="showNodeDetails('${result.node_id}')">
                                ${result.display_text}
                            </div>`
                        ).join('')}
                    `;
                }
            } else {
                searchResults.innerHTML = '<div class="search-no-results">No results found</div>';
            }
        })
        .catch(error => {
            console.error('Search error:', error);
            searchResults.innerHTML = '<div class="search-no-results">Search error occurred</div>';
        });
}

document.addEventListener('DOMContentLoaded', function() {
    modal = document.getElementById('nodeModal');
    modalTitle = document.getElementById('modalTitle');
    modalContent = document.getElementById('modalContent');
    searchResults = document.getElementById('searchResults');
    
    const startDateEl = document.getElementById('startDate');
    const endDateEl = document.getElementById('endDate');
    if ((startDateEl && startDateEl.value) || (endDateEl && endDateEl.value)) {
        const mermaidDiv = document.querySelector('.mermaid');
        if (mermaidDiv) {
            currentMetromap = (mermaidDiv.textContent || mermaidDiv.innerText || '').trim();
            mermaid.run({ querySelector: '.mermaid' }).then(() => {
                setTimeout(addClickEvents, 100);
            }).catch(err => {
                console.error('mermaid render failed on load-skip:', err);
            });
        }
    } else {
        updateMetromap();
    }
    
    const closeBtn = document.getElementsByClassName('close')[0];
    if (closeBtn) {
        closeBtn.onclick = () => modal.style.display = 'none';
    }
    
    window.onclick = function(event) {
        if (event.target == modal) {
            modal.style.display = 'none';
        }
    }
    
    const searchInput = document.getElementById('searchInput');
    const searchButton = document.getElementById('searchButton');
    
    if (searchButton) searchButton.addEventListener('click', performSearch);
    if (searchInput) {
        searchInput.addEventListener('keypress', function(e) {
            if (e.key === 'Enter') {
                performSearch();
            }
        });
        
        searchInput.addEventListener('input', function() {
            if (!searchInput.value.trim()) {
                searchResults.innerHTML = '';
                searchResults.style.display = 'none';
            }
        });
    }

    const filteredButton = document.getElementById('filteredButton');
    if (filteredButton) {
        filteredButton.addEventListener('click', function() {
            fetch('/api/v1/filtered-entries', { cache: 'no-store' })
                .then(r => r.json())
                .then(data => showFilteredModal(data))
                .catch(error => console.error('Error:', error));
        });
    }
    
    const applyButton = document.getElementById('applyTimeRangeButton');
    if (applyButton) {
        applyButton.addEventListener('click', function() {
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
            
            const applyBtn = document.getElementById('applyTimeRangeButton');
            applyBtn.textContent = 'Reloading...';
            applyBtn.disabled = true;
            
            updateMetromap().finally(() => {
                applyBtn.textContent = 'Apply Time Range';
                applyBtn.disabled = false;
            });
        });
    }
    
    const clearButton = document.getElementById('clearTimeRangeButton');
    if (clearButton) {
        clearButton.addEventListener('click', function() {
            const clearBtn = document.getElementById('clearTimeRangeButton');
            clearBtn.textContent = 'Resetting...';
            clearBtn.disabled = true;
            
            document.getElementById('startDate').value = '';
            document.getElementById('startTime').value = '00:00:00';
            document.getElementById('endDate').value = '';
            document.getElementById('endTime').value = '23:59:59';

            updateMetromap(true).finally(() => {
                clearBtn.textContent = 'Reset';
                clearBtn.disabled = false;
            });
        });
    }
});

setInterval(() => {
    const startDate = document.getElementById('startDate')?.value;
    const endDate = document.getElementById('endDate')?.value;
    
    if (!startDate && !endDate) {
        updateMetromap();
    }
}, 10000);