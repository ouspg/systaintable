import argparse
import json
import sys
from datetime import datetime
from flask import Flask, render_template, jsonify, send_from_directory, request
import os
from multiprocessing import Pool, cpu_count

import formation
import nodes

app = Flask(__name__)

PERSONAL_TYPES = {'IP', 'MAC', 'Username', 'Email', 'Hostname', 'URL', 'DNSname', 'TTY'}
TECHNICAL_TYPES = {'dsa', 'asdasd', 'dsadsa'}

current_timeline = ""
node_details = {}
group_merge_log = {}
selected_group_id = None
available_groups = {}
excluded_entries = []

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
    if len(chunk_data) == 5:
        chunk_lines, PERSONAL_TYPES, process_technical_types, common_entries, used_excluded_entries = chunk_data
    else:
        chunk_lines, PERSONAL_TYPES, TECHNICAL_TYPES = chunk_data
        process_technical_types = TECHNICAL_TYPES
        used_excluded_entries = []
        
        try:
            common_values_path = os.path.join('data', 'common_values.txt')
            if not os.path.exists(common_values_path):
                common_values_path = 'common_values.txt'
                
            with open(common_values_path, "r", encoding="utf-8") as f:
                common_entries = set(f.read().splitlines())
        except FileNotFoundError:
            common_entries = set()
    
    local_connections = set()
    local_nodes = set()
    local_node_timestamps = {}
    local_node_counts = {}
    local_node_entries = {}
    local_technical_data = {}

    for line_num, entries in chunk_lines:
        all_line_ids = []
        line_technical_data = []
        
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
            
            is_common = bool(common_entries) and entry_value in common_entries
            is_technical = bool(process_technical_types) and entry_type in process_technical_types
            is_excluded_value = bool(used_excluded_entries) and entry_value in used_excluded_entries
            is_excluded_type = bool(used_excluded_entries) and entry_type in used_excluded_entries

            if is_excluded_value or is_excluded_type:
                tech_entry = {
                    'type': entry_type,
                    'value': entry_value,
                    'timestamp': entry.get('timestamp', 'N/A'),
                    'line': line_num,
                }
                line_technical_data.append(tech_entry)
                continue

            if is_common or is_technical:
                add_identity(entry, entry_type, entry_value)
                continue

            if entry_type in PERSONAL_TYPES:
                add_identity(entry, entry_type, entry_value)
                continue

            tech_entry = {
                'type': entry_type,
                'value': entry_value,
                'timestamp': entry.get('timestamp', 'N/A'),
                'line': line_num,
            }
            line_technical_data.append(tech_entry)
        
        # Yhdistä tekniset tiedot
        if all_line_ids and line_technical_data:
            for node_id in all_line_ids:
                node_key = node_id.split('(')[0]
                if node_key not in local_technical_data:
                    local_technical_data[node_key] = []
                local_technical_data[node_key].extend(line_technical_data)
        
        # Tärkeä connections
        if all_line_ids:
            local_nodes.update(all_line_ids)
            for i in range(len(all_line_ids)):
                for j in range(i+1, len(all_line_ids)):
                    local_connections.add((all_line_ids[i], all_line_ids[j]))
    
    return local_connections, local_nodes, local_node_timestamps, local_node_counts, local_node_entries, local_technical_data

def process_json_file(reload_requested=False, custom_excluded_entries=None, use_multiprocessing=True, start_time=None, end_time=None):
    """Käsittelee JSON-tiedoston ja luo metrokartan"""
    global current_timeline, node_details, available_groups, excluded_entries, group_merge_log
    
    used_excluded_entries = custom_excluded_entries if (reload_requested and custom_excluded_entries is not None) else excluded_entries
    used_excluded_entries = set(used_excluded_entries)
    if start_time or end_time:
        print(f"[process] time filter active start={start_time} end={end_time}")
    print(f"[process] excluded entries active: {len(used_excluded_entries)}")
    
    all_nodes = set()
    connections_set = set()
    node_timestamps = {}
    node_counts = {}
    node_entries = {}
    technical_data = {}

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
            common_values_path = os.path.join('data', 'common_values.txt')
            if not os.path.exists(common_values_path):
                common_values_path = 'common_values.txt'
                
            with open(common_values_path, "r", encoding="utf-8") as f:
                common_entries = set(f.read().splitlines())
                
        except FileNotFoundError:
            common_entries = set()
        
        # Ryhmitellään entryt riveittäin
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
                current_timeline = "flowchart RL\n\n    EmptyResult[No data in time range]"
                return
            else:
                print("File is empty.")
                sys.exit()

        line_items = list(lines.items())
        if use_multiprocessing:
            print(f"Using multiprocessing with {cpu_count()} cores. This may take a while...")
            chunk_size = max(1, len(line_items) // cpu_count())

            process_technical_types = TECHNICAL_TYPES.copy()
            if used_excluded_entries:
                process_technical_types = {t for t in TECHNICAL_TYPES if t not in used_excluded_entries}

            chunks = []
            for i in range(0, len(line_items), chunk_size):
                chunk = line_items[i:i + chunk_size]
                chunks.append((chunk, PERSONAL_TYPES, process_technical_types, common_entries, used_excluded_entries))
            with Pool(processes=cpu_count()) as pool:
                results = pool.map(_process_line_chunk, chunks)
        else:
            print("Processing without multiprocessing...")
            process_technical_types = TECHNICAL_TYPES.copy()
            if used_excluded_entries:
                process_technical_types = {t for t in TECHNICAL_TYPES if t not in used_excluded_entries}
            chunks = [(line_items, PERSONAL_TYPES, process_technical_types, common_entries, used_excluded_entries)]
            results = []
            for chunk in chunks:
                results.append(_process_line_chunk(chunk))
        
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
                first_time = timestamps[0] if timestamps else 'N/A'
                last_time = timestamps[-1] if timestamps else 'N/A'
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
        person_groups, group_merge_log = formation.group_by_person(list(connections_set), node_details)
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
                first_time = all_entries[0]['timestamp'] if all_entries else 'N/A'
                last_time = all_entries[-1]['timestamp'] if all_entries else 'N/A'
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
            
            unique_nodes = set()
            for node in group:
                node_key = node.split('(')[0]
                if node_key in node_details:
                    value = node_details[node_key]['value']
                    node_type = node_details[node_key]['type']
                    unique_nodes.add((value, node_type))
            
            available_groups[group_id] = {
                'count': len(all_entries) + len(all_technical_entries),
                'nodes': len(unique_nodes),
                'first_seen': first_time,
                'last_seen': last_time
            }
        
        if selected_group_id:
            current_timeline = formation.generate_timeline_content(selected_group_id, node_details)
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

@app.route('/api/metromap')
def api_metromap():
    """API metrokartan hakuun (timestamp filtering)"""
    global current_timeline, node_details
    reset = request.args.get('reset')
    start_time_str = request.args.get('start')
    end_time_str = request.args.get('end')

    if reset == '1':
        process_json_file(reload_requested=False, use_multiprocessing=False, start_time=None, end_time=None)
        return jsonify({
            'metromap': current_timeline,
            'timestamp': datetime.now().strftime('%H:%M:%S')
        })

    if not start_time_str and not end_time_str:
        return jsonify({'metromap': current_timeline, 'timestamp': datetime.now().strftime('%H:%M:%S')})

    def _parse_query_dt(raw, label):
        try:
            if 'T' in raw:
                # Sekuntitarkkuus
                if len(raw.split('T')[1].split(':')) == 3:
                    return datetime.strptime(raw, '%Y-%m-%dT%H:%M:%S')
                else:
                    return datetime.strptime(raw, '%Y-%m-%dT%H:%M')
            else:
                return datetime.strptime(raw, '%Y-%m-%d')
        except ValueError:
            print(f"Invalid {label} time format: {raw}")
            return None

    start_dt = _parse_query_dt(start_time_str, 'start') if start_time_str else None
    end_dt = _parse_query_dt(end_time_str, 'end') if end_time_str else None

    if start_dt or end_dt:
        print(f"Time filtering: start={start_dt}, end={end_dt}")
        process_json_file(reload_requested=False, start_time=start_dt, end_time=end_dt, use_multiprocessing=False)
        filtered_result = {
            'metromap': current_timeline,
            'timestamp': datetime.now().strftime('%H:%M:%S')
        }
        return jsonify(filtered_result)
    return jsonify({'metromap': current_timeline, 'timestamp': datetime.now().strftime('%H:%M:%S')})

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
        current_timeline = formation.generate_timeline_content(group_id, node_details)
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
        timeline = formation.generate_timeline_content(group_id, node_details)
        return jsonify({
            'success': True,
            'visualization': timeline,
            'type': 'formation',
            'group_id': group_id
        })
    elif viz_type == 'nodes':
        nodes_diagram = nodes.generate_nodes_content(group_id, node_details)
        return jsonify({
            'success': True,
            'visualization': nodes_diagram,
            'type': 'nodes',
            'group_id': group_id
        })
    elif viz_type == 'timeline':
        return jsonify({
            'success': False,
            'error': 'Chronological timeline visualization not yet made',
            'type': viz_type
        })
    else:
        return jsonify({'error': 'Unknown visualization type'}), 400

@app.route('/api/technical-entries')
def api_technical_entries():
    """API teknisten ja yleisten entryjen hakuun"""
    global excluded_entries
    technical_values = set()
    
    try:
        common_values_path = os.path.join('data', 'common_values.txt')
        if not os.path.exists(common_values_path):
            common_values_path = 'common_values.txt'
            
        with open(common_values_path, "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if line and not line.startswith('#'):
                    technical_values.add(line)
    except FileNotFoundError:
        pass
    
    for node_id, details in node_details.items():
        if details.get('type') != 'Group':
            if 'technical_entries' in details:
                for entry in details['technical_entries']:
                    technical_values.add(entry.get('value', 'Unknown'))
            
            if 'entries' in details:
                for entry in details['entries']:
                    entry_value = entry.get('value', '')
                    entry_type = entry.get('type', '')
                    
                    if entry_type in TECHNICAL_TYPES:
                        technical_values.add(entry_value)
    
    return jsonify(sorted(list(technical_values)))

@app.route('/api/reload-metromap', methods=['POST'])
def reload_metromap():
    """Reload the metromap with custom exclusion settings"""
    global excluded_entries
    try:
        data = request.get_json(silent=True) or {}
        custom_excluded_entries = data.get('excludedEntries', [])
        
        excluded_entries = custom_excluded_entries
        
        process_json_file(reload_requested=True, custom_excluded_entries=custom_excluded_entries, use_multiprocessing=False)
        
        return jsonify({'success': True})
    except Exception as e:
        print(f"Reload error: {e}")
        return jsonify({'success': False, 'error': str(e)})

@app.route('/api/common/add', methods=['POST'])
def api_add_common_entry():
    try:
        data = request.get_json(silent=True) or {}
        value = (data.get('value') or '').strip()
        if not value:
            return jsonify({'success': False, 'message': 'Empty value'}), 400
        if '\n' in value or '\r' in value or len(value) > 500:
            return jsonify({'success': False, 'message': 'Invalid value'}), 400

        common_values_path = os.path.join('data', 'common_values.txt')
        if not os.path.exists(common_values_path):
            common_values_path = 'common_values.txt'

        try:
            with open(common_values_path, 'r', encoding='utf-8') as f:
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

        dirpath = os.path.dirname(common_values_path) or '.'
        os.makedirs(dirpath, exist_ok=True)
        tmp_path = common_values_path + '.tmp'
        with open(tmp_path, 'w', encoding='utf-8') as tf:
            if new_lines:
                tf.write('\n'.join(new_lines).rstrip('\n') + '\n')

        os.replace(tmp_path, common_values_path)

        try:
            with open(common_values_path, 'r', encoding='utf-8') as f:
                excluded_entries_list = [line.strip() for line in f if line.strip() and not line.startswith('#')]
        except Exception:
            excluded_entries_list = []

        global excluded_entries
        excluded_entries = excluded_entries_list

        try:
            process_json_file(reload_requested=True, custom_excluded_entries=excluded_entries, use_multiprocessing=False)
        except Exception as e:
            return jsonify({'success': True, 'action': action, 'message': 'File updated but reprocessing failed: ' + str(e)}), 200

        return jsonify({'success': True, 'action': action})
    except Exception as e:
        print(f"api_common_add error: {e}")
        return jsonify({'success': False, 'message': str(e)}), 500

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

    global excluded_entries
    try:
        common_values_path = os.path.join('data', 'common_values.txt')
        if not os.path.exists(common_values_path):
            common_values_path = 'common_values.txt'
        with open(common_values_path, "r", encoding="utf-8") as f:
            excluded_entries = [line.strip() for line in f if line.strip() and not line.startswith('#')]
    except Exception:
        excluded_entries = []

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
