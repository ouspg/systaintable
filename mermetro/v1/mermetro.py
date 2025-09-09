import json
import sys
import os
from datetime import datetime
from flask import Flask, render_template, jsonify, send_from_directory, request
from multiprocessing import Pool, cpu_count
import re
import string

app = Flask(__name__)

PERSONAL_TYPES = {'IP', 'MAC', 'Username', 'Email', 'Hostname', 'URL', 'DNSname', 'TTY'}
FILTERED_TYPES = {'Example', 'Example2'}

current_metromap = ""
node_details = {}
group_merge_log = {}
filtered_entries = []
startup_multiprocessing = False
lokitiedosto = None

ALLOWED_CHARS = string.ascii_letters + string.digits + '_'
CLEAN_REGEX = re.compile(f"[^{re.escape(ALLOWED_CHARS)}]")
UNDERSCORE_REGEX = re.compile('_+')

def parse_timestamp_to_datetime(timestamp_str):
    """
    Muuntaa timestamp-merkkijonon datetime-objektiksi
    Palauttaa datetimen muotoa (2024, 12, 16, 23, 38, 50)
    """
    if not timestamp_str or timestamp_str == 'N/A':
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
    identities = []
    entry_type = entry['type']
    entry_value = entry['value']
    
    if entry_type == 'URL':
        if '?' in entry_value:
            display_url = entry_value.split('?')[0]
        else:
            display_url = entry_value
        
        display_url = display_url.replace('@', '_AT_').replace('[', '_')
        url_id = UNDERSCORE_REGEX.sub('_', CLEAN_REGEX.sub('_', entry_value))
        identities.append(f'URL_{url_id}([URL<br/>{display_url}])')
    else:
        clean_value = UNDERSCORE_REGEX.sub('_', CLEAN_REGEX.sub('_', entry_value))
        display_value = entry_value.replace('@', '_AT_').replace('[', '_')

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
        
        if entry_type in type_mapping:
            identities.append(type_mapping[entry_type])
    
    return identities

def _process_line_chunk(chunk_data):
    """Funktio prosessoirin"""
    if len(chunk_data) == 5:
        chunk_lines, PERSONAL_TYPES, process_filtered_types, filtered_entries, used_filtered_entries = chunk_data
    else:
        chunk_lines, PERSONAL_TYPES, FILTERED_TYPES = chunk_data
        process_filtered_types = FILTERED_TYPES
        used_filtered_entries = []
        
        try:
            filtered_values_path = os.path.join('data', 'filtered_entries.txt')
            with open(filtered_values_path, "r", encoding="utf-8") as f:
                filtered_entries = set(f.read().splitlines())
        except FileNotFoundError:
            filtered_entries = set()

    local_connections = set()
    local_nodes = set()
    local_node_timestamps = {}
    local_node_counts = {}
    local_node_entries = {}
    local_filtered_data = {}
    
    for line_num, entries in chunk_lines:
        # Käsitellään entry-tasolla sen mukaan onko arvo suodatettu.
        all_line_ids = []
        line_filtered_data = []

        def add_identity(entry, entry_type, entry_value):
            identities = parse_identities(entry)
            if not identities:
                return
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
                    'type': entry_type,
                    'value': entry_value,
                })
        
        for entry in entries:
            if not entry or not isinstance(entry, dict):
                continue
                
            entry_type = entry.get('type')
            entry_value = entry.get('value')
            
            if not entry_type or not entry_value:
                continue

            is_common = bool(filtered_entries) and entry_value in filtered_entries
            is_filtered = bool(process_filtered_types) and entry_type in process_filtered_types
            is_filtered_value = bool(used_filtered_entries) and entry_value in used_filtered_entries
            is_filtered_type = bool(used_filtered_entries) and entry_type in used_filtered_entries

            if is_filtered_value or is_filtered_type:
                filtered_entry = {
                    'type': entry_type,
                    'value': entry_value,
                    'timestamp': entry.get('timestamp', 'N/A'),
                    'line': line_num,
                }
                line_filtered_data.append(filtered_entry)
                continue

            if is_common or is_filtered:
                add_identity(entry, entry_type, entry_value)
                continue

            if entry_type in PERSONAL_TYPES:
                add_identity(entry, entry_type, entry_value)
                continue

            filtered_entry = {
                'type': entry_type,
                'value': entry_value,
                'timestamp': entry.get('timestamp', 'N/A'),
                'line': line_num,
            }
            line_filtered_data.append(filtered_entry)

        # Yhdistä tekniset tiedot
        if all_line_ids and line_filtered_data:
            for node_id in all_line_ids:
                node_key = node_id.split('(')[0]
                if node_key not in local_filtered_data:
                    local_filtered_data[node_key] = []
                local_filtered_data[node_key].extend(line_filtered_data)

        # Tärkeä connections
        if all_line_ids:
            local_nodes.update(all_line_ids)
            for i in range(len(all_line_ids)):
                for j in range(i+1, len(all_line_ids)):
                    local_connections.add((all_line_ids[i], all_line_ids[j]))

    return local_connections, local_nodes, local_node_timestamps, local_node_counts, local_node_entries, local_filtered_data

def group_by_person(connections):
    """
    Luodaan henkilöryhmiä yhteyksien perusteella
    """
    global group_merge_log
    temp_merge_logs = {}

    person_groups = []

    for a, b in connections:
        groups_with_a = []
        groups_with_b = []

        for i, group in enumerate(person_groups):
            if a in group:
                groups_with_a.append(i)
            if b in group:
                groups_with_b.append(i)

        if groups_with_a and groups_with_b:
            if groups_with_a[0] == groups_with_b[0]:
                # Sama ryhmä, ei tehdä mitään uutta
                continue
            else:
                # Eri ryhmät, yhdistetään
                group_a_idx = groups_with_a[0]
                group_b_idx = groups_with_b[0]
                group_a = person_groups[group_a_idx]
                group_b = person_groups[group_b_idx]
                
                # Kerää member-nimet näyttöä varten
                group_a_members = []
                group_b_members = []
                
                for member in group_a:
                    member_key = member.split('(')[0]
                    member_value = node_details.get(member_key, {}).get('value', member_key)
                    if node_details.get(member_key, {}).get('type') == 'URL' and '?' in member_value:
                        member_value = member_value.split('?')[0]
                    group_a_members.append(member_value)
                
                for member in group_b:
                    member_key = member.split('(')[0]
                    member_value = node_details.get(member_key, {}).get('value', member_key)
                    if node_details.get(member_key, {}).get('type') == 'URL' and '?' in member_value:
                        member_value = member_value.split('?')[0]
                    group_b_members.append(member_value)
                
                # Yhdistä ryhmät
                group_a.update(group_b)
                
                # Yhdistä merge-logit (A:n logit + B:n logit + tämä yhdistäminen)
                a_logs = temp_merge_logs.get(group_a_idx, [])
                b_logs = temp_merge_logs.get(group_b_idx, [])
                
                a_key = a.split('(')[0]
                b_key = b.split('(')[0]
                a_value = node_details.get(a_key, {}).get('value', a_key)
                b_value = node_details.get(b_key, {}).get('value', b_key)
                
                if node_details.get(a_key, {}).get('type') == 'URL' and '?' in a_value:
                    a_value = a_value.split('?')[0]
                if node_details.get(b_key, {}).get('type') == 'URL' and '?' in b_value:
                    b_value = b_value.split('?')[0]
                
                group_a_str = ", ".join(group_a_members)
                group_b_str = ", ".join(group_b_members)
                
                detailed_log = f"MERGED: Group A [{group_a_str}]\n + \nGroup B [{group_b_str}] \nbecause of tuple ({a_value} , {b_value})"
                
                # Yhdistä kaikki logit
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
            # Vain a on ryhmässä, lisätään b
            idx = groups_with_a[0]
            person_groups[idx].add(b)
            
            if idx not in temp_merge_logs:
                temp_merge_logs[idx] = []
                
            a_key = a.split('(')[0]
            b_key = b.split('(')[0]
            a_value = node_details.get(a_key, {}).get('value', a_key)
            b_value = node_details.get(b_key, {}).get('value', b_key)
            
            if node_details.get(a_key, {}).get('type') == 'URL' and '?' in a_value:
                a_value = a_value.split('?')[0]
            if node_details.get(b_key, {}).get('type') == 'URL' and '?' in b_value:
                b_value = b_value.split('?')[0]
                
            temp_merge_logs[idx].append(f"JOINED: ({a_value} , {b_value}) -> added to group")
            
        elif groups_with_b:
            # Vain b on ryhmässä, lisätään a
            idx = groups_with_b[0]
            person_groups[idx].add(a)
            
            if idx not in temp_merge_logs:
                temp_merge_logs[idx] = []
                
            a_key = a.split('(')[0]
            b_key = b.split('(')[0]
            a_value = node_details.get(a_key, {}).get('value', a_key)
            b_value = node_details.get(b_key, {}).get('value', b_key)
            
            if node_details.get(a_key, {}).get('type') == 'URL' and '?' in a_value:
                a_value = a_value.split('?')[0]
            if node_details.get(b_key, {}).get('type') == 'URL' and '?' in b_value:
                b_value = b_value.split('?')[0]
                
            temp_merge_logs[idx].append(f"JOINED: ({b_value} , {a_value}) -> added to group")
            
        else:
            # Kumpikaan ei ole ryhmässä, luodaan uusi
            person_groups.append({a, b})
            current_idx = len(person_groups) - 1
            
            a_key = a.split('(')[0]
            b_key = b.split('(')[0]
            a_value = node_details.get(a_key, {}).get('value', a_key)
            b_value = node_details.get(b_key, {}).get('value', b_key)
            
            if node_details.get(a_key, {}).get('type') == 'URL' and '?' in a_value:
                a_value = a_value.split('?')[0]
            if node_details.get(b_key, {}).get('type') == 'URL' and '?' in b_value:
                b_value = b_value.split('?')[0]
                
            temp_merge_logs[current_idx] = [f"FORMED: ({a_value} , {b_value}) = new group"]

    # Lopuksi luo lopulliset ID:t ja merge_log
    group_merge_log = {}
    for i, group in enumerate(person_groups):
        final_group_id = f"ID_{i + 1}"
        group_merge_log[final_group_id] = temp_merge_logs.get(i, [])

    return person_groups

def generate_metromap_content(all_nodes, connections):
    """Luo ja palauttaa Mermaidin syntaksilla metrokartan sisällön"""
    content = "flowchart RL\n\n"
    
    person_groups = group_by_person(connections)
    processed_nodes = set()
    
    colors = [
        ("#4CAF50", "#2E7D32"), ("#2196F3", "#1565C0"), ("#9C27B0", "#6A1B9A"),
        ("#FF9800", "#E65100"), ("#F44336", "#C62828"), ("#795548", "#3E2723"),
        ("#607D8B", "#263238"), ("#E91E63", "#AD1457")
    ]
    
    all_connections = []
    all_styles = []
    
    for group_num, group in enumerate(person_groups):
        group_list = list(group)
        color_index = group_num % len(colors)
        fill_color, stroke_color = colors[color_index]
        
        # Yksittäinen solmu
        if len(group_list) == 1:
            node = group_list[0]
            node_id = node.split('(')[0]
            group_ball_id = f"ID_{group_num + 1}"
            group_ball_node = f"{group_ball_id}(({group_ball_id}))"
            
            content += f"    {node}\n"
            content += f"    {group_ball_node}\n"
            content += "\n"
            
            all_connections.append(f"    {group_ball_id} --- {node_id}")
            
            all_styles.append(f"    style {node_id} fill:{fill_color},stroke:{stroke_color},color:#fff,stroke-width:3px")
            all_styles.append(f"    style {group_ball_id} fill:{fill_color},stroke:{stroke_color},color:#fff,stroke-width:4px,stroke-dasharray: 3 3")
            all_styles.append(f"    linkStyle {len(all_connections) - 1} stroke:{fill_color},stroke-width:5px")
            
            processed_nodes.add(node)
        else:
            # Useamman solmun ryhmä
            group_ball_id = f"ID_{group_num + 1}"
            group_ball_node = f"{group_ball_id}(({group_ball_id}))"
            
            for node in group_list:
                content += f"    {node}\n"
            content += f"    {group_ball_node}\n"
            content += "\n"
            
            # Ensimmäiselle tasolle 3 solmua
            first_level = group_list[:3]
            remaining_nodes = group_list[3:]
            for node in first_level:
                node_id = node.split('(')[0]
                all_connections.append(f"    {group_ball_id} --- {node_id}")
            
            current_level = first_level
            remaining = remaining_nodes
            
            # Loopissa muodostetaan metrokartan kolmirivinen rakenne
            while current_level and remaining:
                next_level = []
                for i, parent_node in enumerate(current_level):
                    if i < len(remaining):
                        child_node = remaining[i]
                        parent_id = parent_node.split('(')[0]
                        child_id = child_node.split('(')[0]
                        all_connections.append(f"    {parent_id} --- {child_id}")
                        next_level.append(child_node)
                
                current_level = next_level
                remaining = remaining[len(next_level):]
            
            for node in group_list:
                node_id = node.split('(')[0]
                all_styles.append(f"    style {node_id} fill:{fill_color},stroke:{stroke_color},color:#fff,stroke-width:3px")
            
            all_styles.append(f"    style {group_ball_id} fill:{fill_color},stroke:{stroke_color},color:#fff,stroke-width:4px,stroke-dasharray: 3 3")
            
            # Link-tyylit
            total_connections = len(group_list)
            connection_start = len(all_connections) - total_connections
            for i in range(total_connections):
                all_styles.append(f"    linkStyle {connection_start + i} stroke:{fill_color},stroke-width:5px")
            
            processed_nodes.update(group_list)
    
    # Lisää jäljelle jääneet solmut omina ryhminä
    remaining_nodes = all_nodes - processed_nodes
    for i, node in enumerate(sorted(remaining_nodes)):
        group_num = len(person_groups) + i
        color_index = group_num % len(colors)
        fill_color, stroke_color = colors[color_index]
        
        node_id = node.split('(')[0]
        group_ball_id = f"ID_{group_num + 1}"
        group_ball_node = f"{group_ball_id}(({group_ball_id}))"
        
        content += f"    {node}\n"
        content += f"    {group_ball_node}\n"
        content += "\n"

        all_connections.append(f"    {group_ball_id} --- {node_id}")

        all_styles.append(f"    style {node_id} fill:{fill_color},stroke:{stroke_color},color:#fff,stroke-width:3px")
        all_styles.append(f"    style {group_ball_id} fill:{fill_color},stroke:{stroke_color},color:#fff,stroke-width:4px,stroke-dasharray: 3 3")
        all_styles.append(f"    linkStyle {len(all_connections) - 1} stroke:{fill_color},stroke-width:5px")
    
    # Lisää yhteydet ja tyylit
    content += "\n    %% Connections\n"
    for connection in all_connections:
        content += f"{connection}\n"
    
    content += "\n    %% Styles\n"
    for style in all_styles:
        content += f"{style}\n"
    
    return content

def process_json_file(reload_requested=False, custom_filtered_entries=None, use_multiprocessing=True, start_time=None, end_time=None):
    """Käsittelee JSON-tiedoston ja luo metrokartan"""
    global current_metromap, node_details, filtered_entries
    
    used_filtered_entries = custom_filtered_entries if (reload_requested and custom_filtered_entries is not None) else filtered_entries
    used_filtered_entries = set(used_filtered_entries)
    if start_time or end_time:
        print(f"[process] time filter active start={start_time} end={end_time}")
    print(f"[process] filtered entries active: {len(used_filtered_entries)}")
    
    all_nodes = set()
    connections_set = set()
    node_timestamps = {}
    node_counts = {}
    node_entries = {}
    filtered_data = {}

    def _entry_in_time(e):
        if not (start_time or end_time):
            return True
        entry_timestamp = parse_timestamp_to_datetime(e.get('timestamp', 'N/A'))
        if not entry_timestamp:
            return False
        if start_time and entry_timestamp < start_time:
            return False
        if end_time and entry_timestamp > end_time:
            return False
        return True
    
    try:
        with open(lokitiedosto, "r", encoding="utf-8") as f:
            data = json.load(f)
        
        try:
            filtered_values_path = os.path.join('data', 'filtered_entries.txt')
            with open(filtered_values_path, "r", encoding="utf-8") as f:
                filtered_entries = set(f.read().splitlines())
                
        except FileNotFoundError:
            filtered_entries = set()

        lines = {}
        total_entries = len(data)
        filtered_entries_count = 0
        
        for entry in data:
            if 'line' in entry:
                include_entry = True
                if start_time or end_time:
                    include_entry = _entry_in_time(entry)
                
                if include_entry:
                    filtered_entries_count += 1
                    line_num = entry['line']
                    if line_num not in lines:
                        lines[line_num] = []
                    lines[line_num].append(entry)

        if start_time or end_time:
            print(f"Time filtering: {total_entries} total entries -> {filtered_entries_count} entries match time range")
            print(f"Time range: {start_time} to {end_time}")
            
        print(f"Processing {len(lines)} lines...")
        if len(lines) == 0:
            if start_time or end_time:
                print("No entries found in the specified time range.")
                current_metromap = "flowchart RL\n\n    EmptyResult[No data in time range]"
                return
            else:
                print("File is empty.")
                sys.exit()

        line_items = list(lines.items())
        if use_multiprocessing:
            print(f"Using multiprocessing with {cpu_count()} cores. This may take a while...")
            chunk_size = max(1, len(line_items) // cpu_count())
            process_filtered_types = FILTERED_TYPES.copy()
            if used_filtered_entries:
                process_filtered_types = {t for t in FILTERED_TYPES if t not in used_filtered_entries}

            chunks = []
            for i in range(0, len(line_items), chunk_size):
                chunk = line_items[i:i + chunk_size]
                chunks.append((chunk, PERSONAL_TYPES, process_filtered_types, filtered_entries, used_filtered_entries))
            with Pool(processes=cpu_count()) as pool:
                results = pool.map(_process_line_chunk, chunks)
        else:
            print("Processing without multiprocessing...")
            process_filtered_types = FILTERED_TYPES.copy()
            if used_filtered_entries:
                process_filtered_types = {t for t in FILTERED_TYPES if t not in used_filtered_entries}
            chunks = [(line_items, PERSONAL_TYPES, process_filtered_types, filtered_entries, used_filtered_entries)]
            results = []
            for chunk in chunks:
                results.append(_process_line_chunk(chunk))

        for chunk_connections, chunk_nodes, chunk_timestamps, chunk_counts, chunk_entries, chunk_filtered in results:
            connections_set.update(chunk_connections)
            all_nodes.update(chunk_nodes)
            
            for node_key, timestamp_list in chunk_timestamps.items():
                if node_key not in node_timestamps:
                    node_timestamps[node_key] = []
                node_timestamps[node_key].extend(timestamp_list)
            
            for node_key, count in chunk_counts.items():
                node_counts[node_key] = node_counts.get(node_key, 0) + count
                
            for node_key, entry_list in chunk_entries.items():
                if node_key not in node_entries:
                    node_entries[node_key] = []
                node_entries[node_key].extend(entry_list)

            for node_key, filtered_list in chunk_filtered.items():
                if node_key not in filtered_data:
                    filtered_data[node_key] = []
                filtered_data[node_key].extend(filtered_list)

        connections = list(connections_set)
        
        updated_nodes = set()
        local_node_details = {}
        
        for node in all_nodes:
            node_key = node.split('(')[0]
            timestamps = node_timestamps.get(node_key, ['N/A'])
            count = node_counts.get(node_key, 0)
            entries = node_entries.get(node_key, [])
            filtered_entries = filtered_data.get(node_key, [])

            valid_timestamps = []
            for ts in timestamps:
                dt = parse_timestamp_to_datetime(ts)
                if dt:
                    valid_timestamps.append(dt)
            
            if valid_timestamps:
                valid_timestamps.sort()
                first_time = timestamps[0] if timestamps else 'N/A'
                last_time = timestamps[-1] if timestamps else 'N/A'
                
                local_node_details[node_key] = {
                    'type': entries[0]['type'] if entries else 'Unknown',
                    'value': entries[0]['value'] if entries else 'Unknown',
                    'count': count,
                    'first_seen': first_time,
                    'last_seen': last_time,
                    'entries': sorted(entries, key=lambda x: x['timestamp']),
                    'filtered_entries': sorted(filtered_entries, key=lambda x: x['timestamp'])
                }

                if first_time == last_time:
                    info = f"<br/>Time: {first_time}<br/>Count: {count}"
                else:
                    info = f"<br/>First: {first_time}<br/>Last: {last_time}<br/>Count: {count}"
            else:
                local_node_details[node_key] = {
                    'type': entries[0]['type'] if entries else 'Unknown',
                    'value': entries[0]['value'] if entries else 'Unknown',
                    'count': count,
                    'first_seen': 'N/A',
                    'last_seen': 'N/A',
                    'entries': entries,
                    'filtered_entries': filtered_entries
                }
                info = f"<br/>Time: N/A<br/>Count: {count}"
            
            if '([' in node and node.endswith('])'):
                parts = node.split('([')
                if len(parts) == 2:
                    node_id = parts[0]
                    content = parts[1][:-2]
                    updated_node = f"{node_id}([{content}{info}])"
                    updated_nodes.add(updated_node)
                else:
                    updated_nodes.add(node)
            else:
                updated_nodes.add(node)
        
        updated_connections = []
        for a, b in connections:
            a_key = a.split('(')[0]
            b_key = b.split('(')[0]
            
            updated_a = next((node for node in updated_nodes if node.split('(')[0] == a_key), None)
            updated_b = next((node for node in updated_nodes if node.split('(')[0] == b_key), None)
            
            if updated_a and updated_b:
                updated_connections.append((updated_a, updated_b))
        
        # Luodaan ryhmäpalloon tiedot
        person_groups = group_by_person(updated_connections)
        for group_num, group in enumerate(person_groups):
            group_id = f"ID_{group_num + 1}"
            
            all_entries = []
            all_filtered_entries = []
            all_timestamps = []
            
            for node in group:
                node_key = node.split('(')[0]
                if node_key in local_node_details:
                    all_entries.extend(local_node_details[node_key]['entries'])
                    all_filtered_entries.extend(local_node_details[node_key]['filtered_entries'])

                    for entry in local_node_details[node_key]['entries']:
                        dt = parse_timestamp_to_datetime(entry['timestamp'])
                        if dt:
                            all_timestamps.append(dt)
            
            all_entries.sort(key=lambda x: x['line'])
            all_filtered_entries.sort(key=lambda x: x['line'])

            if all_timestamps:
                all_timestamps.sort()
                first_time = all_entries[0]['timestamp'] if all_entries else 'N/A'
                last_time = all_entries[-1]['timestamp'] if all_entries else 'N/A'
            else:
                first_time = 'N/A'
                last_time = 'N/A'
            
            local_node_details[group_id] = {
                'type': 'Group',
                'value': f"{len(group)} entries",
                'count': len(all_entries) + len(all_filtered_entries),
                'first_seen': first_time,
                'last_seen': last_time,
                'entries': all_entries,
                'filtered_entries': all_filtered_entries,
                'group_nodes': list(group),
                'merge_log': group_merge_log.get(group_id, [])
            }
        
        node_details.clear()
        node_details.update(local_node_details)
        
        current_metromap = generate_metromap_content(updated_nodes, updated_connections)
        print(f"Metromap updated: {datetime.now().strftime('%H:%M:%S')}")
        
    except Exception as e:
        print(f"JSON Error: {e}")

@app.route('/')
def index():
    """Pääsivu"""
    return render_template('sivusto.html', 
                                  metromap=current_metromap,
                                  timestamp=datetime.now().strftime('%H:%M:%S'))

@app.route('/api/v1/metromap')
def api_metromap():
    """API metrokartan hakuun"""
    global current_metromap, node_details
    reset = request.args.get('reset')
    start_time_str = request.args.get('start')
    end_time_str = request.args.get('end')

    if reset == '1':
        process_json_file(reload_requested=False, use_multiprocessing=False, start_time=None, end_time=None)
        return jsonify({
            'metromap': current_metromap,
            'timestamp': datetime.now().strftime('%H:%M:%S')
        })

    if not start_time_str and not end_time_str:
        return jsonify({'metromap': current_metromap, 'timestamp': datetime.now().strftime('%H:%M:%S')})

    def _parse_query_dt(raw, label):
        try:
            if 'T' in raw:
                return datetime.strptime(raw, '%Y-%m-%dT%H:%M:%S')
            else:
                return datetime.strptime(raw, '%Y-%m-%d')
        except ValueError:
            print(f"Invalid {label} time format: {raw}")
            return None

    start_dt = _parse_query_dt(start_time_str, 'start') if start_time_str else None
    end_dt = _parse_query_dt(end_time_str, 'end') if end_time_str else None

    if start_dt or end_dt:
        print(f"Time filtering: start={start_dt}, end={end_dt}")
        process_json_file(reload_requested=False, start_time=start_dt, end_time=end_dt, use_multiprocessing=startup_multiprocessing)
        filtered_result = {
            'metromap': current_metromap,
            'timestamp': datetime.now().strftime('%H:%M:%S')
        }
        return jsonify(filtered_result)
    return jsonify({'metromap': current_metromap, 'timestamp': datetime.now().strftime('%H:%M:%S')})

@app.route('/api/v1/node-details/<node_id>')
def api_node_details(node_id):
    """API entryn tietojen hakemiseen"""
    if node_id in node_details:
        return jsonify(node_details[node_id])
    clean_node_id = node_id
    if '-' in node_id:
        parts = node_id.split('-')
        if parts[-1].isdigit():
            clean_node_id = '-'.join(parts[:-1])
    if clean_node_id in node_details:
        return jsonify(node_details[clean_node_id])
    for key in node_details.keys():
        if (key.replace('_', '-') == clean_node_id or 
            key.replace('-', '_') == clean_node_id or 
            clean_node_id.replace('_', '-') == key or 
            clean_node_id.replace('-', '_') == key):
            return jsonify(node_details[key])
    return jsonify({'error': 'Node not found'}), 404

@app.route('/api/v1/search/<search_term>')
def api_search(search_term):
    """API hakutoiminnolle"""
    results = []
    search_lower = search_term.lower()
    
    is_timestamp_search = search_term.startswith('"') and search_term.endswith('"')
    is_combined_search = ' "' in search_term and search_term.endswith('"')

    if is_timestamp_search:
        clean_search_term = search_term[1:-1]
    elif is_combined_search:
        parts = search_term.split(' "')
        text_part = parts[0].lower()
        time_part = parts[1][:-1]
    else:
        clean_search_term = search_term
    
    for node_id, details in node_details.items():
        if details.get('type', '') != 'Group':
            if not is_timestamp_search:
                search_value = text_part if is_combined_search else search_lower
                if search_value in details.get('value', '').lower():
                    display_text = f"{details.get('type', 'Unknown')}: {details.get('value', 'Unknown')}"
                    results.append({
                        'node_id': node_id,
                        'display_text': display_text,
                        'match_type': 'primary'
                    })
                    continue
            
        found_in_entries = False
        if 'entries' in details:
            for entry in details['entries']:
                match_found = False

                # formatted_time jätetään jos halutaan lisätä se switch, (iso tai finnish time)
                entry_time = entry.get('formatted_time', entry.get('timestamp', ''))
                entry_value = entry.get('value', '').lower()

                if is_timestamp_search:
                    if clean_search_term in entry_time:
                        match_found = True
                elif is_combined_search:
                    if text_part in entry_value and time_part in entry_time:
                        match_found = True
                else:
                    if search_lower in entry_value:
                        match_found = True

                if match_found:
                    if is_timestamp_search:
                        if details.get('type') == 'Group':
                            display_text = f"{node_id} (Match in group)"
                        elif '-' in clean_search_term and ':' in clean_search_term:
                            display_text = f"{details.get('type', 'Unknown')}: {details.get('value', 'Unknown')} (Full timestamp match: {entry_time})"
                        elif '-' in clean_search_term and ':' not in clean_search_term:
                            display_text = f"{details.get('type', 'Unknown')}: {details.get('value', 'Unknown')} (Date match: {entry_time})"
                        elif ':' in clean_search_term and '-' not in clean_search_term:
                            display_text = f"{details.get('type', 'Unknown')}: {details.get('value', 'Unknown')} (Time match: {entry_time})"
                        else:
                            display_text = f"{details.get('type', 'Unknown')}: {details.get('value', 'Unknown')} (Timestamp match: {entry_time})"
                    elif is_combined_search:
                        display_text = f"{details.get('type', 'Unknown')}: {node_id} (Combined match: {entry_value} at {entry_time})"
                    else:
                        display_text = f"{node_id} (part of group)"
                    results.append({
                        'node_id': node_id,
                        'display_text': display_text,
                        'match_type': 'entry'
                    })
                    found_in_entries = True
                    break

        if not found_in_entries and 'filtered_entries' in details:
            for entry in details['filtered_entries']:
                match_found = False
                
                # formatted_time jätetään jos halutaan lisätä se switch, (iso tai finnish time)
                entry_time = entry.get('formatted_time', entry.get('timestamp', ''))
                entry_value = entry.get('value', '').lower()

                if is_timestamp_search:
                    if clean_search_term in entry_time:
                        match_found = True
                elif is_combined_search:
                    if text_part in entry_value and time_part in entry_time:
                        match_found = True
                else:
                    if search_lower in entry_value:
                        match_found = True

                if match_found:
                    if is_timestamp_search:
                        if '.' in clean_search_term and ':' in clean_search_term:
                            display_text = f"{details.get('type', 'Unknown')}: {details.get('value', 'Unknown')} (Full timestamp match: {entry_time} (filtered entry))"
                        elif '.' in clean_search_term and ':' not in clean_search_term:
                            display_text = f"{details.get('type', 'Unknown')}: {details.get('value', 'Unknown')} (Date match: {entry_time} (filtered entry))"
                        elif ':' in clean_search_term and '.' not in clean_search_term:
                            display_text = f"{details.get('type', 'Unknown')}: {details.get('value', 'Unknown')} (Time match: {entry_time} (filtered entry))"
                    elif is_combined_search:
                        display_text = f"{details.get('type', 'Unknown')}: {details.get('value', 'Unknown')} (Combined match: {text_part} at {time_part} in filtered)"
                    else:
                        display_text = f"{details.get('type', 'Unknown')}: {details.get('value', 'Unknown')} (found in filtered entries)"
                    
                    results.append({
                        'node_id': node_id,
                        'display_text': display_text,
                        'match_type': 'filtered'
                    })
                    break
    
    unique_results = []
    seen_nodes = set()
    for result in results:
        if result['node_id'] not in seen_nodes:
            unique_results.append(result)
            seen_nodes.add(result['node_id'])
    
    unique_results.sort(key=lambda x: (x['match_type'] != 'primary', x['display_text']))
    
    return jsonify({'results': unique_results})

@app.route('/favicon.ico')
def favicon():
    """Pikku ikoni välilehdessä"""
    return send_from_directory(os.path.join(app.root_path, 'static'),
                               'favicon.ico', mimetype='image/vnd.microsoft.icon')

@app.route('/api/v1/filtered-entries')
def api_filtered_entries():
    """API suodattamien entryjen hakuun"""
    global filtered_entries
    filtered_values = set()

    try:
        filtered_values_path = os.path.join('data', 'filtered_entries.txt')
        with open(filtered_values_path, "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if line and not line.startswith('#'):
                    filtered_values.add(line)
    except FileNotFoundError:
        pass
    
    for node_id, details in node_details.items():
        if details.get('type') != 'Group':
            if 'filtered_entries' in details:
                for entry in details['filtered_entries']:
                    filtered_values.add(entry.get('value', 'Unknown'))

            if 'entries' in details:
                for entry in details['entries']:
                    entry_value = entry.get('value', '')
                    entry_type = entry.get('type', '')
                    
                    if entry_type in FILTERED_TYPES:
                        filtered_values.add(entry_value)

    return jsonify(sorted(list(filtered_values)))

@app.route('/api/v1/reload-metromap', methods=['POST'])
def reload_metromap():
    """Reload the metromap with custom filtered settings"""
    global filtered_entries
    try:
        data = request.get_json(silent=True) or {}
        custom_filtered_entries = data.get('filteredEntries', [])

        print(f"[reload] multiprocessing={startup_multiprocessing} filtered={len(custom_filtered_entries)}")
        filtered_entries = custom_filtered_entries

        process_json_file(
            reload_requested=True, 
            custom_filtered_entries=filtered_entries, 
            use_multiprocessing=startup_multiprocessing
        )
        
        return jsonify({'success': True})
    except Exception as e:
        print(f"Reload error: {e}")
        return jsonify({'success': False, 'error': str(e)}), 500

@app.route('/api/v1/common/add', methods=['POST'])
def api_add_common_entry():
    try:
        data = request.get_json(silent=True) or {}
        value = (data.get('value') or '').strip()
        if not value:
            return jsonify({'success': False, 'message': 'Empty value'}), 400
        if '\n' in value or '\r' in value or len(value) > 500:
            return jsonify({'success': False, 'message': 'Invalid value'}), 400

        filtered_values_path = os.path.join('data', 'filtered_entries.txt')

        try:
            with open(filtered_values_path, 'r', encoding='utf-8') as f:
                lines = f.read().splitlines()
        except FileNotFoundError:
            lines = []

        existing_values = {line.strip() for line in lines if line.strip() and not line.strip().startswith('#')}

        if value in existing_values:
            new_lines = []
            for line in lines:
                stripped = line.strip()
                if stripped and not stripped.startswith('#') and stripped == value:
                    continue
                new_lines.append(line)
            action = 'removed'
        else:
            new_lines = lines + [value]
            action = 'added'

        os.makedirs('data', exist_ok=True)
        tmp_path = filtered_values_path + '.tmp'
        with open(tmp_path, 'w', encoding='utf-8') as tf:
            if new_lines:
                tf.write('\n'.join(new_lines).rstrip('\n') + '\n')

        os.replace(tmp_path, filtered_values_path)

        try:
            with open(filtered_values_path, 'r', encoding='utf-8') as f:
                filtered_entries_list = [line.strip() for line in f if line.strip() and not line.startswith('#')]
        except Exception:
            filtered_entries_list = []

        global filtered_entries
        filtered_entries = filtered_entries_list

        return jsonify({'success': True, 'action': action})
    except Exception as e:
        print(f"Error adding filtered entry: {e}")
        return jsonify({'success': False, 'error': str(e)}), 500

def start_app(jsonfile, multiprocessing=False, host='127.0.0.1', port=5000):

    global lokitiedosto, startup_multiprocessing, filtered_entries

    if not os.path.isfile(jsonfile):
        print(f"Error: File '{jsonfile}' not found.")
        return

    lokitiedosto = jsonfile
    startup_multiprocessing = bool(multiprocessing)

    try:
        filtered_values_path = os.path.join('data', 'filtered_entries.txt')
        with open(filtered_values_path, "r", encoding="utf-8") as f:
            filtered_entries = [line.strip() for line in f if line.strip() and not line.startswith('#')]
    except Exception:
        filtered_entries = []

    process_json_file(use_multiprocessing=startup_multiprocessing)

    try:
        os.makedirs('data', exist_ok=True)
        with open('data/metrokartta_koodi.txt', 'w', encoding='utf-8') as f:
            f.write(current_metromap)
    except Exception as e:
        print(f"Warning writing output files: {e}")

    try:
        app.run(debug=False, host=host, port=port)
    except KeyboardInterrupt:
        print("\nStopped by user")
