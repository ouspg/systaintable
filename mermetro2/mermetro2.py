import argparse
import json
import sys
from datetime import datetime
from flask import Flask, render_template, jsonify, send_from_directory
import os
from multiprocessing import Pool, cpu_count

app = Flask(__name__)

PERSONAL_TYPES = {'IP', 'MAC', 'Username', 'Email', 'Hostname', 'URL', 'DNSname'}
TECHNICAL_TYPES = {'TTY', 'dsa', 'asdasd', 'dsadsa'}

current_timeline = ""
node_details = {}
group_merge_log = {}
selected_group_id = None
available_groups = {}
    
def parse_timestamp_to_datetime(timestamp_str):
    """Muuntaa timestamp-merkkijonon datetime-objektiksi"""
    if timestamp_str == 'N/A':
        return None
    
    try:
        # 2024-12-16 23:38:50
        if len(timestamp_str) == 19 and '-' in timestamp_str and ':' in timestamp_str:
            return datetime.strptime(timestamp_str, '%Y-%m-%d %H:%M:%S')
        
        # Dec 16 23:38:50
        if len(timestamp_str.split()) == 3:
            current_year = datetime.now().year
            timestamp_with_year = f"{current_year} {timestamp_str}"
            return datetime.strptime(timestamp_with_year, '%Y %b %d %H:%M:%S')
        
        return None
    except Exception:
        return None

def parse_identities(entry):
    """Muuttaa entryt mermaid-koodiksi"""
    entry_type = entry['type']
    entry_value = entry['value']
    
    replacements = {
        " ": "_", "@": "_AT_", "%": "_", ":": "_", "[": "_", "]": "_",
        ".": "_", "=": "_", "/": "_", "?": "_", "(": "_", ")": "_",
        "~": "_", "&": "_", "#": "_"
    }
    
    clean_value = entry_value
    for old, new in replacements.items():
        clean_value = clean_value.replace(old, new)
    display_value = entry_value.replace('@', '_AT_').replace('[', '_')
    
    if entry_type == 'URL':
        if '?' in entry_value:
            display_url = entry_value.split('?')[0]
        else:
            display_url = entry_value
        
        display_url = display_url.replace('@', '_AT_').replace('[', '_')
        url_id = entry_value.replace("://", "_").replace("/", "_").replace("?","_").replace("~","_").replace("&","_").replace("=","_").replace("#","_").replace("%","_")
        return [f'URL_{url_id}([URL<br/>{display_url}])']
    
    type_mapping = {
        'IP': f'IPv4_{clean_value}([IP-Address<br/>{display_value}])',
        'DNSname': f'DNS_{clean_value}([DNSname<br/>{display_value}])',
        'MAC-osoite': f'MAC_{clean_value}([MAC-Address<br/>{display_value}])',
        'Username': f'User_{clean_value}([User<br/>{display_value}])',
        'Email': f'Email_{clean_value}([Email<br/>{display_value}])',
        'Hostname': f'Hostname_{clean_value}([Hostname<br/>{display_value}])',
        'TTY': f'TTY_{clean_value}([TTY<br/>{display_value}])',
        'Example': f'Example_{clean_value}([Example<br/>{display_value}])',
        'Example2': f'Example2_{clean_value}([Example2<br/>{display_value}])',
    }
    
    return [type_mapping.get(entry_type, f'{entry_type}_{clean_value}([{entry_type}<br/>{display_value}])')]

def _process_line_chunk(chunk_data):
    """Prosessoi yhden riviryhmän rinnakkain"""
    chunk_lines, PERSONAL_TYPES, TECHNICAL_TYPES = chunk_data
    local_connections = set()
    local_nodes = set()
    local_node_timestamps = {}
    local_node_counts = {}
    local_node_entries = {}
    local_technical_data = {}

    try:
        with open("data/common_values.txt", "r", encoding="utf-8") as f:
            common_entries = set(f.read().splitlines())
    except FileNotFoundError:
        common_entries = set()

    for line_num, entries in chunk_lines:
        all_line_ids = []
        line_technical_data = []
        
        for entry in entries:
            entry_type = entry['type']
            entry_value = entry['value']
            
            if entry_value in common_entries or entry_type in TECHNICAL_TYPES:
                tech_entry = {
                    'type': entry_type,
                    'value': entry_value,
                    'timestamp': entry.get('timestamp', 'N/A'),
                    'line': line_num,
                }
                line_technical_data.append(tech_entry)
            elif entry_type in PERSONAL_TYPES:
                identities = parse_identities(entry)
                all_line_ids.extend(identities)
                
                for node_id in identities:
                    node_key = node_id.split('(')[0]
                    timestamp = entry.get('timestamp', 'N/A')
                    
                    if node_key not in local_node_timestamps:
                        local_node_timestamps[node_key] = []
                        local_node_counts[node_key] = 0
                        local_node_entries[node_key] = []
                    
                    local_node_timestamps[node_key].append(timestamp)
                    local_node_counts[node_key] += 1
                    local_node_entries[node_key].append({
                        'timestamp': timestamp,
                        'line': line_num,
                        'type': entry['type'],
                        'value': entry['value'],
                    })
        
        if all_line_ids and line_technical_data:
            for node_id in all_line_ids:
                node_key = node_id.split('(')[0]
                if node_key not in local_technical_data:
                    local_technical_data[node_key] = []
                local_technical_data[node_key].extend(line_technical_data)
        
        if all_line_ids:
            local_nodes.update(all_line_ids)
            for i in range(len(all_line_ids)):
                for j in range(i+1, len(all_line_ids)):
                    local_connections.add((all_line_ids[i], all_line_ids[j]))
    
    return local_connections, local_nodes, local_node_timestamps, local_node_counts, local_node_entries, local_technical_data

def get_node_value(node_key):
    """Hakee noden arvon node_details:sta"""
    if node_key in node_details:
        member_value = node_details[node_key].get('value', node_key)
        if node_details[node_key].get('type') == 'URL' and '?' in member_value:
            return member_value.split('?')[0]
        return member_value
    return node_key

def group_by_person(connections):
    """Luodaan henkilöryhmiä yhteyksien perusteella"""
    global group_merge_log
    temp_merge_logs = {}
    person_groups = []

    for a, b in connections:
        groups_with_a = [i for i, group in enumerate(person_groups) if a in group]
        groups_with_b = [i for i, group in enumerate(person_groups) if b in group]

        if groups_with_a and groups_with_b:
            if groups_with_a[0] == groups_with_b[0]:
                continue
            else:
                # Yhdistetään ryhmät
                group_a_idx = groups_with_a[0]
                group_b_idx = groups_with_b[0]
                group_a = person_groups[group_a_idx]
                group_b = person_groups[group_b_idx]
                
                group_a_members = [get_node_value(member.split('(')[0]) for member in group_a]
                group_b_members = [get_node_value(member.split('(')[0]) for member in group_b]
                
                group_a.update(group_b)
                
                a_logs = temp_merge_logs.get(group_a_idx, [])
                b_logs = temp_merge_logs.get(group_b_idx, [])
                
                a_value = get_node_value(a.split('(')[0])
                b_value = get_node_value(b.split('(')[0])
                
                group_a_str = ", ".join(group_a_members)
                group_b_str = ", ".join(group_b_members)
                detailed_log = f"MERGED: Group A [{group_a_str}]\n + \nGroup B [{group_b_str}] \nbecause of Tuple ({a_value} , {b_value})"

                combined_logs = a_logs + b_logs + [detailed_log]
                temp_merge_logs[group_a_idx] = combined_logs

                if group_b_idx in temp_merge_logs:
                    del temp_merge_logs[group_b_idx]

                del person_groups[group_b_idx]

                new_temp_logs = {}
                for idx, logs in temp_merge_logs.items():
                    new_idx = idx if idx < group_b_idx else idx - 1
                    new_temp_logs[new_idx] = logs
                temp_merge_logs = new_temp_logs
                    
        elif groups_with_a:
            # Lisää b ryhmään A
            idx = groups_with_a[0]
            person_groups[idx].add(b)
            
            if idx not in temp_merge_logs:
                temp_merge_logs[idx] = []
                
            a_value = get_node_value(a.split('(')[0])
            b_value = get_node_value(b.split('(')[0])
            temp_merge_logs[idx].append(f"ADDED: ({a_value} , {b_value}) -> added to group")
            
        elif groups_with_b:
            # Lisää a ryhmään B
            idx = groups_with_b[0]
            person_groups[idx].add(a)
            
            if idx not in temp_merge_logs:
                temp_merge_logs[idx] = []
                
            a_value = get_node_value(a.split('(')[0])
            b_value = get_node_value(b.split('(')[0])
            temp_merge_logs[idx].append(f"ADDED: ({a_value} , {b_value}) -> added to group")
            
        else:
            # Luo uusi ryhmä
            person_groups.append({a, b})
            current_idx = len(person_groups) - 1
            
            a_value = get_node_value(a.split('(')[0])
            b_value = get_node_value(b.split('(')[0])
            temp_merge_logs[current_idx] = [f"FORMED: ({a_value} , {b_value}) = new group"]

    group_merge_log = {}
    for i, group in enumerate(person_groups):
        final_group_id = f"ID_{i + 1}"
        group_merge_log[final_group_id] = temp_merge_logs.get(i, [])

    return person_groups

def create_formed_from_data(group_id, val1, val2):
    group_entries = node_details[group_id].get('entries', []) if group_id in node_details else []
    
    line_to_values = {}
    line_to_entries = {}
    for entry in group_entries:
        line = entry.get('line', None)
        if line is not None:
            line_to_values.setdefault(line, set()).add(entry.get('value'))
            line_to_entries.setdefault(line, []).append(entry)
    
    # Etsi rivi jolla molemmat val1 ja val2 esiintyvät
    found_line = None
    for line, valueset in line_to_values.items():
        if val1 in valueset and val2 in valueset:
            found_line = line
            break
    
    formed_from = []
    if found_line is not None:
        # Hae molempien tiedot samalta riviltä
        for v in (val1, val2):
            entry = next((e for e in line_to_entries[found_line] if e.get('value') == v), None)
            if entry:
                formed_from.append({
                    'value': v,
                    'line': found_line,
                    'timestamp': entry.get('timestamp', 'N/A'),
                    'type': entry.get('type', 'Unknown'),
                    'tuple_line': found_line
                })
            else:
                formed_from.append({
                    'value': v,
                    'line': found_line,
                    'timestamp': 'N/A',
                    'type': 'Unknown',
                    'tuple_line': found_line
                })
    else:
        for val in (val1, val2):
            entries = [e for e in group_entries if e.get('value') == val]
            if entries:
                entry = entries[0]
                formed_from.append({
                    'value': val,
                    'line': entry.get('line', 'N/A'),
                    'timestamp': entry.get('timestamp', 'N/A'),
                    'type': entry.get('type', '')
                })
            else:
                formed_from.append({
                    'value': val,
                    'line': 'N/A',
                    'timestamp': 'N/A',
                    'type': ''
                })
    
    return formed_from

def generate_timeline_content(group_id):
    if group_id not in node_details:
        return "flowchart TD\n    ERROR[Group not found]"
    
    group_data = node_details[group_id]
    merge_logs = group_data.get('merge_log', [])
    
    if not merge_logs:
        return "flowchart TD\n    ERROR[No formation history available]"
    
    content = "flowchart TD\n"
    group_counter = 0
    node_definitions = []
    connections = []
    group_nodes = []
    group_id_map = {}

    colors = [
        ("#4CAF50", "#2E7D32"), ("#2196F3", "#1565C0"), ("#9C27B0", "#6A1B9A"),
        ("#FF9800", "#E65100"), ("#F44336", "#C62828"), ("#795548", "#3E2723"),
        ("#607D8B", "#263238"), ("#E91E63", "#AD1457")
    ]
    
    color_assignments = {}
    next_color_index = 0
    parent_groups = {}

    for log_entry in merge_logs:
        if "FORMED:" in log_entry:
            group_counter += 1
            start_idx = log_entry.find("(") + 1
            end_idx = log_entry.find(")")
            tuple_content = log_entry[start_idx:end_idx]
            values = [v.strip() for v in tuple_content.split(",")]
            
            if len(values) >= 2:
                val1, val2 = values[0], values[1]
                members = frozenset([val1, val2])
                group_node_id = f"G{group_counter}_FORMED"
                group_label = f"{val1}<br>{val2}"
                node_definitions.append(f"    {group_node_id}(\"{group_label}\"):::node")
                group_nodes.append((members, group_node_id))
                group_id_map[members] = group_node_id

                color_assignments[group_node_id] = next_color_index
                next_color_index = (next_color_index + 1) % len(colors)

                formed_from = create_formed_from_data(group_id, val1, val2)
                
                node_details[group_node_id] = {
                    'type': 'GroupFormed',
                    'value': f"FORMED: ({val1}, {val2})",
                    'formed_from': formed_from,
                    'merge_log': [log_entry]
                }
                
        elif "ADDED:" in log_entry:
            group_counter += 1
            start_idx = log_entry.find("(") + 1
            end_idx = log_entry.find(")")
            tuple_content = log_entry[start_idx:end_idx]
            values = [v.strip() for v in tuple_content.split(",")]
            
            if len(values) >= 2:
                val1, val2 = values[0], values[1]
                prev_members = None
                prev_node_id = None
                
                for members, node_id in reversed(group_nodes):
                    if val1 in members or val2 in members:
                        prev_members = members
                        prev_node_id = node_id
                        break
                
                new_members = frozenset(set(prev_members) | {val1, val2})
                group_node_id = f"G{group_counter}_ADDED"
                group_label = "<br>".join(sorted(new_members))
                node_definitions.append(f"    {group_node_id}(\"{group_label}\"):::node")
                group_nodes.append((new_members, group_node_id))
                group_id_map[new_members] = group_node_id
                
                parent_groups[group_node_id] = prev_node_id
                
                color_assignments[group_node_id] = color_assignments.get(prev_node_id, next_color_index)
                if prev_node_id not in color_assignments:
                    next_color_index = (next_color_index + 1) % len(colors)
                
                connections.append(f"    {prev_node_id} -- \"{val1}<br>{val2}\" --> {group_node_id}")

                formed_from = create_formed_from_data(group_id, val1, val2)
                current_entries = [e for e in node_details[group_id].get('entries', []) if e.get('value') in new_members]
                
                node_details[group_node_id] = {
                    'type': 'GroupAdded',
                    'value': f"ADDED: ({val1}, {val2})",
                    'formed_from': formed_from,
                    'merge_log': [log_entry],
                    'entries': current_entries
                }

        elif "MERGED:" in log_entry:
            group_counter += 1
            group_a_start = log_entry.find("Group A [") + 9
            group_a_end = log_entry.find("]", group_a_start)
            group_a_content = log_entry[group_a_start:group_a_end]
            group_b_start = log_entry.find("Group B [") + 9
            group_b_end = log_entry.find("]", group_b_start)
            group_b_content = log_entry[group_b_start:group_b_end]
            tuple_start = log_entry.find("because of Tuple (") + 18
            tuple_end = log_entry.find(")", tuple_start)
            connecting_tuple = log_entry[tuple_start:tuple_end] if tuple_start > 17 and tuple_end > tuple_start else "Unknown"
            connecting_tuple = "<br>".join([v.strip() for v in connecting_tuple.split(",")])
            
            group_a_members = frozenset([m.strip() for m in group_a_content.split(",") if m.strip()])
            group_b_members = frozenset([m.strip() for m in group_b_content.split(",") if m.strip()])
            merged_members = frozenset(set(group_a_members) | set(group_b_members))
            group_node_id = f"G{group_counter}_MERGED"
            group_label = "<br>".join(sorted(merged_members))
            node_definitions.append(f"    {group_node_id}(\"{group_label}\"):::node")
            group_nodes.append((merged_members, group_node_id))
            group_id_map[merged_members] = group_node_id
            
            node_a = group_id_map.get(group_a_members)
            node_b = group_id_map.get(group_b_members)
            
            color_assignments[group_node_id] = next_color_index
            next_color_index = (next_color_index + 1) % len(colors)
            
            if node_a:
                connections.append(f"    {node_a} -- \"{connecting_tuple}\" --> {group_node_id}")
                parent_groups[group_node_id] = node_a
            if node_b:
                connections.append(f"    {node_b} -- \"{connecting_tuple}\" --> {group_node_id}")
            
            group_entries = node_details[group_id].get('entries', []) if group_id in node_details else []
            
            # Luo uniikit entryt ryhmille
            def get_unique_entries(members):
                unique_entries = []
                seen_values = set()
                for e in group_entries:
                    if e.get('value') in members and e.get('value') not in seen_values:
                        unique_entries.append(e)
                        seen_values.add(e.get('value'))
                return unique_entries
            
            unique_group_a_entries = get_unique_entries(group_a_members)
            unique_group_b_entries = get_unique_entries(group_b_members)
            merged_entries = [e for e in group_entries if e.get('value') in merged_members]

            # Merging tuple details
            merging_tuple = []
            original_tuple_values = [v.strip() for v in log_entry[tuple_start:tuple_end].split(",") if v.strip()]
            
            for v in original_tuple_values:
                entry = next((e for e in group_entries if e.get('value') == v.strip()), None)
                if entry:
                    merging_tuple.append({
                        'value': entry.get('value'),
                        'line': entry.get('line', ''),
                        'timestamp': entry.get('timestamp', ''),
                        'type': entry.get('type', '')
                    })
                else:
                    merging_tuple.append({
                        'value': v.strip(),
                        'line': '',
                        'timestamp': '',
                        'type': ''
                    })
                    
            node_details[group_node_id] = {
                'type': 'GroupMerged',
                'value': f"MERGED: ({', '.join(sorted(merged_members))})",
                'merged_groups': [
                    {'entries': unique_group_a_entries},
                    {'entries': unique_group_b_entries}
                ],
                'entries': merged_entries,
                'merge_log': [log_entry],
                'merging_tuple': merging_tuple
            }

    content += "\n".join(node_definitions)
    content += "\n\n"
    content += "\n".join(connections)
    content += "\n\n"
    
    for idx, (members, node_id) in enumerate(group_nodes):
        color_index = color_assignments.get(node_id, idx % len(colors))
        fill_color, stroke_color = colors[color_index]
        content += f"    style {node_id} fill:{fill_color},stroke:{stroke_color},color:#fff,stroke-width:3px\n"

    for idx, conn in enumerate(connections):
        parts = conn.strip().split(" -- ")
        if len(parts) == 2:
            source_node = parts[0].strip()
            if source_node in color_assignments:
                color_index = color_assignments[source_node]
                fill_color, stroke_color = colors[color_index]
                content += f"    linkStyle {idx} stroke:{fill_color},stroke-width:4px\n"

    content += "    classDef group fill:#e8f5e8,stroke:#2e7d32,stroke-width:2px;\n"
    return content

def process_json_file(use_multiprocessing=True):
    """Käsittelee JSON-tiedoston ja luo metrokartan"""
    global current_timeline, node_details, available_groups
    
    all_nodes = set()
    connections_set = set()
    node_timestamps = {}
    node_counts = {}
    node_entries = {}
    technical_data = {}
    
    try:
        with open(lokitiedosto, "r", encoding="utf-8") as f:
            data = json.load(f)
        
        # Ryhmitellään entryt riveittäin
        lines = {}
        for entry in data:
            if 'line' in entry:
                line_num = entry['line']
                lines.setdefault(line_num, []).append(entry)

        print(f"Processing {len(lines)} lines...")
        if len(lines) == 0:
            print("File is empty!")
            sys.exit()

        line_items = list(lines.items())
        if use_multiprocessing:
            print(f"Using multiprocessing with {cpu_count()} cores. This may take a while...")
            chunk_size = max(1, len(line_items) // cpu_count())
            chunks = [(line_items[i:i + chunk_size], PERSONAL_TYPES, TECHNICAL_TYPES) 
                     for i in range(0, len(line_items), chunk_size)]
            
            with Pool(processes=cpu_count()) as pool:
                results = pool.map(_process_line_chunk, chunks)
        else:
            print("Processing without multiprocessing...")
            results = [_process_line_chunk((line_items, PERSONAL_TYPES, TECHNICAL_TYPES))]

        # Yhdistä tulokset
        for chunk_connections, chunk_nodes, chunk_timestamps, chunk_counts, chunk_entries, chunk_technical in results:
            connections_set.update(chunk_connections)
            all_nodes.update(chunk_nodes)
            
            for node_key, data_list in chunk_timestamps.items():
                node_timestamps.setdefault(node_key, []).extend(data_list)
            for node_key, count in chunk_counts.items():
                node_counts[node_key] = node_counts.get(node_key, 0) + count
            for node_key, data_list in chunk_entries.items():
                node_entries.setdefault(node_key, []).extend(data_list)
            for node_key, data_list in chunk_technical.items():
                technical_data.setdefault(node_key, []).extend(data_list)
        
        print(f"Processing completed. Found {len(connections_set)} connections.")

        # Luo node_details
        node_details = {}
        for node in all_nodes:
            node_key = node.split('(')[0]
            timestamps = node_timestamps.get(node_key, ['N/A'])
            count = node_counts.get(node_key, 0)
            entries = node_entries.get(node_key, [])
            tech_entries = technical_data.get(node_key, [])
            
            actual_value = entries[0]['value'] if entries else 'Unknown'
            actual_type = entries[0]['type'] if entries else 'Unknown'
            
            valid_timestamps = [parse_timestamp_to_datetime(ts) for ts in timestamps if parse_timestamp_to_datetime(ts)]
            
            if valid_timestamps:
                valid_timestamps.sort()
                first_time = valid_timestamps[0].strftime('%d.%m at %H:%M:%S')
                last_time = valid_timestamps[-1].strftime('%d.%m at %H:%M:%S')
            else:
                first_time = last_time = 'N/A'
                
            node_details[node_key] = {
                'type': actual_type,
                'value': actual_value,
                'count': count,
                'first_seen': first_time,
                'last_seen': last_time,
                'entries': sorted(entries, key=lambda x: x['timestamp']),
                'technical_entries': sorted(tech_entries, key=lambda x: x['timestamp'])
            }
        
        # Luodaan ryhmätiedot
        person_groups = group_by_person(list(connections_set))
        available_groups = {}
        
        for group_num, group in enumerate(person_groups):
            group_id = f"ID_{group_num + 1}"
            
            all_entries = []
            all_technical_entries = []
            all_timestamps = []
            
            for node in group:
                node_key = node.split('(')[0]
                if node_key in node_details:
                    details = node_details[node_key]
                    all_entries.extend(details['entries'])
                    all_technical_entries.extend(details['technical_entries'])
                    
                    for entry in details['entries']:
                        dt = parse_timestamp_to_datetime(entry['timestamp'])
                        if dt:
                            all_timestamps.append(dt)
            
            all_entries.sort(key=lambda x: x['line'])
            all_technical_entries.sort(key=lambda x: x['line'])
            
            if all_timestamps:
                all_timestamps.sort()
                first_time = all_timestamps[0].strftime('%d.%m.at %H:%M:%S')
                last_time = all_timestamps[-1].strftime('%d.%m.at %H:%M:%S')
            else:
                first_time = last_time = 'N/A'
            
            node_details[group_id] = {
                'type': 'Group',
                'value': f"{len(group)} entries",
                'count': len(all_entries) + len(all_technical_entries),
                'first_seen': first_time,
                'last_seen': last_time,
                'entries': all_entries,
                'technical_entries': all_technical_entries,
                'group_nodes': list(group),
                'merge_log': group_merge_log.get(group_id, [])
            }
            
            available_groups[group_id] = {
                'count': len(all_entries) + len(all_technical_entries),
                'nodes': len(group),
                'first_seen': first_time,
                'last_seen': last_time
            }
        
        if selected_group_id:
            current_timeline = generate_timeline_content(selected_group_id)
            print(f"Timeline updated for {selected_group_id}: {datetime.now().strftime('%H:%M:%S')}")
        
    except Exception as e:
        print(f"JSON Error: {e}")

@app.route('/')
def index():
    """Pääsivu"""
    return render_template('group_timeline.html', 
                          timeline=current_timeline,
                          selected_group=selected_group_id,
                          available_groups=available_groups,
                          timestamp=datetime.now().strftime('%H:%M:%S'))

@app.route('/api/timeline')
def api_timeline():
    """API timeline-sisällön hakuun"""
    return jsonify({
        'timeline': current_timeline, 
        'selected_group': selected_group_id,
        'timestamp': datetime.now().strftime('%H:%M:%S')
    })

@app.route('/api/groups')
def api_groups():
    """API ryhmien hakuun"""
    return jsonify(available_groups)

@app.route('/api/select-group/<group_id>')
def api_select_group(group_id):
    """API ryhmän valintaan"""
    global selected_group_id, current_timeline
    
    if group_id in available_groups:
        selected_group_id = group_id
        current_timeline = generate_timeline_content(group_id)
        return jsonify({
            'success': True, 
            'selected_group': selected_group_id,
            'timeline': current_timeline
        })
    else:
        return jsonify({'success': False, 'error': 'Group not found'}), 404

@app.route('/api/node-details/<node_id>')
def api_node_details(node_id):
    """API entryn tietojen hakemiseen"""
    if node_id.startswith("flowchart-"):
        node_id = node_id.replace("flowchart-", "")

    node_id_clean = node_id.split('-')[0]

    if node_id in node_details:
        return jsonify(node_details[node_id])
    if node_id_clean in node_details:
        return jsonify(node_details[node_id_clean])
    return jsonify({'error': 'Node not found'}), 404

@app.route('/favicon.ico')
def favicon():
    """Pikku ikoni välilehdessä"""
    return send_from_directory(os.path.join(app.root_path, 'static'),
                               'favicon.ico', mimetype='image/vnd.microsoft.icon')

@app.route('/api/visualization/<viz_type>/<group_id>')
def api_visualization(viz_type, group_id):
    """API eri visualisointityyppien hakemiseen"""
    if group_id not in available_groups:
        return jsonify({'error': 'Group not found'}), 404
    
    if viz_type == 'formation':
        timeline = generate_timeline_content(group_id)
        return jsonify({
            'success': True,
            'visualization': timeline,
            'type': 'formation',
            'group_id': group_id
        })
    elif viz_type in ['nodes', 'timeline']:
        viz_names = {'nodes': 'Node relationship', 'timeline': 'Chronological timeline'}
        return jsonify({
            'success': False,
            'error': f'{viz_names[viz_type]} visualization not yet made',
            'type': viz_type
        })
    else:
        return jsonify({'error': 'Unknown visualization type'}), 400

def main():
    parser = argparse.ArgumentParser(
        description="Example: python3 mermetro2.py data/lokitiedosto.json -m"
    )
    parser.add_argument("jsonfile", help="Path to the log JSON file")
    parser.add_argument("-m", "--multiprocessing", action="store_true", help="Enable multiprocessing for processing")
    args = parser.parse_args()
    
    if not os.path.isfile(args.jsonfile):
        print(f"Error: File '{args.jsonfile}' not found.\n")
        parser.print_help()
        exit(1)

    global lokitiedosto
    lokitiedosto = args.jsonfile

    print(f"\nStarting up...\nTime: {datetime.now().strftime('%d.%m.%Y %H:%M:%S')}")
    
    process_json_file(use_multiprocessing=args.multiprocessing)

    print(f"Found {len(available_groups)} groups")
    print("Groups will be selectable on the web interface.")
    print(f"\nAvailable Groups: {', '.join(available_groups.keys())}")
    print("\nAccess:\n   Live-page: http://localhost:5001")
    
    try:
        app.run(debug=False, host='127.0.0.1', port=5001)
    except KeyboardInterrupt:
        print("\nStopped by user")

if __name__ == "__main__":
    main()
