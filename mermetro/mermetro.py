import json
import time
import sys
from pytz import timezone
from datetime import datetime
from flask import Flask, render_template, jsonify, send_from_directory
from watchdog.observers import Observer
from watchdog.events import FileSystemEventHandler
import os

app = Flask(__name__)

PERSONAL_TYPES = {'IP', 'MAC-osoite', 'Username', 'Email', 'Hostname', 'URL', 'DNSname'}
TECHNICAL_TYPES = {'TTY', 'dsa', 'asdasd', 'dsadsa'}

current_metromap = ""
node_details = {}
group_merge_log = {}

def find_json():
    """Etsii JSON-tiedoston, joka sisältää metrokartan"""
    global lokitiedosto
    json_files = [f for f in os.listdir('.') if f.endswith('.json')]
    if not json_files:
        print("No .json files were found")
        sys.exit()
    print("Found .json files:")
    print("(0) Exit")
    for idx, fname in enumerate(json_files, 1):
        print(f"({idx}) {fname}")
        time.sleep(0.1)
    while True:
        try:
            answer = int(input("Choose a log file (a number): "))
            if answer == 0:
                print("Shutting down")
                sys.exit()
            if 1 <= answer <= len(json_files):
                lokitiedosto = json_files[answer - 1]
                print(f"Using file: {lokitiedosto}")
                break
            else:
                print("Invalid choice, please choose a number from the list.")
        except ValueError:
            print("Invalid input, please enter a number.")

def is_common(value):
    """Tarkistaa, onko arvo common_values.txt-tiedostossa"""
    try:
        with open("common_values.txt", "r", encoding="utf-8") as f:
            common_entries = f.read().splitlines()
        return value in common_entries
    except FileNotFoundError:
        return False

def convert_to_finnish_time(timestamp_str):
    """
    Muuntaa erilaiset timestampit Suomen aikaan
    Palauttaa merkkijonon muotoa "16.12 at 23:38:50"
    """
    if timestamp_str == 'N/A':
        return 'N/A'
    
    try:
        # 2024-12-16 23:38:50
        if len(timestamp_str) == 19 and '-' in timestamp_str and ':' in timestamp_str:
            dt = datetime.strptime(timestamp_str, '%Y-%m-%d %H:%M:%S')
            return dt.strftime('%d.%m at %H:%M:%S')
        
        # Dec 16 23:38:50
        if len(timestamp_str.split()) == 3:
            current_year = datetime.now().year
            timestamp_with_year = f"{current_year} {timestamp_str}"
            dt = datetime.strptime(timestamp_with_year, '%Y %b %d %H:%M:%S')
            return dt.strftime('%d.%m at %H:%M:%S')
        
        # ISO formaatti UTC
        utc_dt = datetime.fromisoformat(timestamp_str.replace('Z', '+00:00'))
        finnish_tz = timezone('Europe/Helsinki')
        finnish_dt = utc_dt.astimezone(finnish_tz)
        return finnish_dt.strftime('%d.%m.%Y at %H:%M:%S')
        
    except Exception:
        return timestamp_str
    
def parse_timestamp_to_datetime(timestamp_str):
    """
    Muuntaa timestamp-merkkijonon datetime-objektiksi
    Palauttaa datetimen muotoa (2024, 12, 16, 23, 38, 50)
    """
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
    """Muuntaa lokitiedoston tunnisteet Mermaid-koodiksi"""
    identities = []
    entry_type = entry['type']
    value = entry['value']
    
    if entry_type == 'URL':
        if '?' in value:
            display_url = value.split('?')[0]
        else:
            display_url = value
        
        display_url = display_url.replace('@', '_AT_').replace('[', '_')
        
        url_id = value.replace("://", "_").replace("/", "_").replace("?","_").replace("~","_").replace("&","_").replace("=","_").replace("#","_").replace("%","_")
        
        identities.append(f'URL_{url_id}([URL<br/>{display_url}])')
        return identities
    else:
        replacements = {
        " ": "_", "@": "_AT_", "%": "_", ":": "_", "[": "_", "]": "_",
        ".": "_", "=": "_", "/": "_", "?": "_", "(": "_", ")": "_",
        "~": "_", "&": "_", "#": "_"
        }
        
        clean_value = value
        for old, new in replacements.items():
            clean_value = clean_value.replace(old, new)
        value = value.replace('@', '_AT_').replace('[', '_')

    type_mapping = {
        'IP': f'IPv4_{clean_value}([IP-Address<br/>{value}])',
        'DNSname': f'DNS_{clean_value}([DNSname<br/>{value}])',
        'MAC-osoite': f'MAC_{clean_value}([MAC-Address<br/>{value}])',
        'Username': f'User_{clean_value}([User<br/>{value}])',
        'Email': f'Email_{clean_value}([Email<br/>{value}])',
        'Hostname': f'Hostname_{clean_value}([Hostname<br/>{value}])',
        'TTY': f'TTY_{clean_value}([TTY<br/>{value}])',
        'Example': f'Example_{clean_value}([Example<br/>{value}])',
        'Example2': f'Example2_{clean_value}([Example2<br/>{value}])',
    }
    
    if entry_type in type_mapping:
        identities.append(type_mapping[entry_type])
    
    return identities

def group_by_person(connections):
    """
    Luodaan henkilöryhmiä yhteyksien perusteella
    """
    global group_merge_log
    group_merge_log = {}

    person_groups = []
    group_counter = 0

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
                group_id = f"ID_{groups_with_a[0] + 1}"
                if group_id not in group_merge_log:
                    group_merge_log[group_id] = []
                a_key = a.split('(')[0]
                b_key = b.split('(')[0]
                a_value = node_details.get(a_key, {}).get('value', a_key)
                b_value = node_details.get(b_key, {}).get('value', b_key)
                
                if node_details.get(a_key, {}).get('type') == 'URL' and '?' in a_value:
                    a_value = a_value.split('?')[0]
                if node_details.get(b_key, {}).get('type') == 'URL' and '?' in b_value:
                    b_value = b_value.split('?')[0]
                    
                continue
            else:
                # Eri ryhmät, yhdistetään
                group_a_idx = groups_with_a[0]
                group_b_idx = groups_with_b[0]
                group_a = person_groups[group_a_idx]
                group_b = person_groups[group_b_idx]
                
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
                
                group_a.update(group_b)
                del person_groups[group_b_idx]
                group_id = f"ID_{group_a_idx + 1}"
                if group_id not in group_merge_log:
                    group_merge_log[group_id] = []
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
                group_merge_log[group_id].append(detailed_log)
                
                print(f"MERGED GROUPS {group_id}:")
                print(f"  Reason: {a_value} <-> {b_value}")
                print(f"  Group A had: {group_a_str}")
                print(f"  Group B had: {group_b_str}")
                print(f"  Result: All combined into {group_id}")
                print("-" * 50)
                    
        elif groups_with_a:
            # Vain a on ryhmässä, lisätään b
            idx = groups_with_a[0]
            person_groups[idx].add(b)
            group_id = f"ID_{idx + 1}"
            if group_id not in group_merge_log:
                group_merge_log[group_id] = []
            a_key = a.split('(')[0]
            b_key = b.split('(')[0]
            a_value = node_details.get(a_key, {}).get('value', a_key)
            b_value = node_details.get(b_key, {}).get('value', b_key)
            
            if node_details.get(a_key, {}).get('type') == 'URL' and '?' in a_value:
                a_value = a_value.split('?')[0]
            if node_details.get(b_key, {}).get('type') == 'URL' and '?' in b_value:
                b_value = b_value.split('?')[0]
                
            group_merge_log[group_id].append(f"ADDED: ({a_value} , {b_value}) -> added to group")
            print(f"{a_value} and {b_value} joined group {group_id}")
        elif groups_with_b:
            # Vain b on ryhmässä, lisätään a
            idx = groups_with_b[0]
            person_groups[idx].add(a)
            group_id = f"ID_{idx + 1}"
            if group_id not in group_merge_log:
                group_merge_log[group_id] = []
            a_key = a.split('(')[0]
            b_key = b.split('(')[0]
            a_value = node_details.get(a_key, {}).get('value', a_key)
            b_value = node_details.get(b_key, {}).get('value', b_key)
            
            if node_details.get(a_key, {}).get('type') == 'URL' and '?' in a_value:
                a_value = a_value.split('?')[0]
            if node_details.get(b_key, {}).get('type') == 'URL' and '?' in b_value:
                b_value = b_value.split('?')[0]
                
            group_merge_log[group_id].append(f"ADDED: ({a_value} , {b_value}) -> added to group")
            print(f"{a_value} and {b_value} joined group {group_id}")
        else:
            # Kumpikaan ei ole ryhmässä, luodaan uusi
            person_groups.append({a, b})
            group_counter += 1
            group_id = f"ID_{group_counter}"
            group_merge_log[group_id] = []
            a_key = a.split('(')[0]
            b_key = b.split('(')[0]
            a_value = node_details.get(a_key, {}).get('value', a_key)
            b_value = node_details.get(b_key, {}).get('value', b_key)
            
            # Lyhennä URL:t jos ne on liian pitkiä
            if node_details.get(a_key, {}).get('type') == 'URL' and '?' in a_value:
                a_value = a_value.split('?')[0]
            if node_details.get(b_key, {}).get('type') == 'URL' and '?' in b_value:
                b_value = b_value.split('?')[0]
                
            group_merge_log[group_id].append(f"FORMED: ({a_value} , {b_value}) = new group")
            print(f"{a_value} and {b_value} formed a group {group_id}")

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

def process_json_file():
    """Käsittelee JSON-tiedoston ja luo metrokartan"""
    global current_metromap, node_details
    
    all_nodes = set()
    connections_set = set()  # Muutettu: käytetään settiä alusta alkaen
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
                if line_num not in lines:
                    lines[line_num] = []
                lines[line_num].append(entry)

        print(f"Processing {len(lines)} lines...")
        if len(lines) == 0:
            print("File is empty.")
            sys.exit()

        # Käsitellään rivit
        for line_num, entries in lines.items():
            all_line_ids = []
            line_technical_data = []
            
            for entry in entries:
                entry_type = entry['type']
                entry_value = entry['value']
                
                if is_common(entry_value) or entry_type in TECHNICAL_TYPES:
                    tech_entry = {
                        'type': entry_type,
                        'value': entry_value,
                        'timestamp': entry.get('timestamp', 'N/A'),
                        'line': line_num,
                        'formatted_time': convert_to_finnish_time(entry.get('timestamp', 'N/A'))
                    }
                    line_technical_data.append(tech_entry)
                    
                elif entry_type in PERSONAL_TYPES:
                    # Normaalit tiedot
                    ids = parse_identities(entry)
                    all_line_ids.extend(ids)
                    
                    for node_id in ids:
                        node_key = node_id.split('(')[0]
                        timestamp = entry.get('timestamp', 'N/A')
                        
                        # Luodaan ja lisätään tietoja sanakirjoihin
                        if node_key not in node_timestamps:
                            node_timestamps[node_key] = []
                            node_counts[node_key] = 0
                            node_entries[node_key] = []
                        
                        node_timestamps[node_key].append(timestamp)
                        node_counts[node_key] += 1
                        node_entries[node_key].append({
                            'timestamp': timestamp,
                            'line': line_num,
                            'type': entry['type'],
                            'value': entry['value'],
                            'formatted_time': convert_to_finnish_time(timestamp)
                        })

            # Yhdistetään tekniset tiedot henkilötietoihin
            if all_line_ids and line_technical_data:
                for node_id in all_line_ids:
                    node_key = node_id.split('(')[0]
                    if node_key not in technical_data:
                        technical_data[node_key] = []
                    technical_data[node_key].extend(line_technical_data)
            
            #Tärkeä connections
            if all_line_ids:
                all_nodes.update(all_line_ids)
                for i in range(len(all_line_ids)):
                    for j in range(i+1, len(all_line_ids)):
                        connections_set.add((all_line_ids[i], all_line_ids[j]))
                        print(f"{line_num}/{len(lines)} Luotu tuple ({all_line_ids[i]}) , ({all_line_ids[j]})")
        
        connections = list(connections_set)
        
        updated_nodes = set()
        node_details = {}
        
        for node in all_nodes:
            node_key = node.split('(')[0]
            timestamps = node_timestamps.get(node_key, ['N/A'])
            count = node_counts.get(node_key, 0)
            entries = node_entries.get(node_key, [])
            tech_entries = technical_data.get(node_key, [])
            
            valid_timestamps = []
            for ts in timestamps:
                dt = parse_timestamp_to_datetime(ts)
                if dt:
                    valid_timestamps.append(dt)
            
            if valid_timestamps:
                valid_timestamps.sort()
                first_time = valid_timestamps[0].strftime('%d.%m at %H:%M:%S')
                last_time = valid_timestamps[-1].strftime('%d.%m at %H:%M:%S')
                
                node_details[node_key] = {
                    'type': entries[0]['type'] if entries else 'Unknown',
                    'value': entries[0]['value'] if entries else 'Unknown',
                    'count': count,
                    'first_seen': first_time,
                    'last_seen': last_time,
                    'entries': sorted(entries, key=lambda x: x['formatted_time']),
                    'technical_entries': sorted(tech_entries, key=lambda x: x['formatted_time'])
                }   
                
                if first_time == last_time:
                    info = f"<br/>Time: {first_time}<br/>Count: {count}"
                else:
                    info = f"<br/>Oldest: {first_time}<br/>Latest: {last_time}<br/>Count: {count}"
            else:
                node_details[node_key] = {
                    'type': entries[0]['type'] if entries else 'Unknown',
                    'value': entries[0]['value'] if entries else 'Unknown',
                    'count': count,
                    'first_seen': 'N/A',
                    'last_seen': 'N/A',
                    'entries': entries,
                    'technical_entries': tech_entries
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
            all_technical_entries = []
            all_timestamps = []
            
            for node in group:
                node_key = node.split('(')[0]
                if node_key in node_details:
                    all_entries.extend(node_details[node_key]['entries'])
                    all_technical_entries.extend(node_details[node_key]['technical_entries'])
                    
                    for entry in node_details[node_key]['entries']:
                        dt = parse_timestamp_to_datetime(entry['timestamp'])
                        if dt:
                            all_timestamps.append(dt)
            
            all_entries.sort(key=lambda x: x['line'])
            all_technical_entries.sort(key=lambda x: x['line'])
            
            if all_timestamps:
                all_timestamps.sort()
                first_time = all_timestamps[0].strftime('%d.%m at %H:%M:%S')
                last_time = all_timestamps[-1].strftime('%d.%m at %H:%M:%S')
            else:
                first_time = 'N/A'
                last_time = 'N/A'
            
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
        
        current_metromap = generate_metromap_content(updated_nodes, updated_connections)
        print(f"Metromap updated: {datetime.now().strftime('%H:%M:%S')}")
        
    except Exception as e:
        print(f"JSON Error: {e}")

class JSONFileHandler(FileSystemEventHandler):
    """Tiedoston seurantaluokka"""
    def on_modified(self, event):
        if event.src_path.endswith(lokitiedosto):
            print("JSON-file modified, updating metromap...")
            time.sleep(0.2)
            process_json_file()

@app.route('/')
def index():
    """Pääsivu"""
    return render_template('sivusto.html', 
                                  metromap=current_metromap,
                                  timestamp=datetime.now().strftime('%H:%M:%S'))

@app.route('/api/metromap')
def api_metromap():
    """API metrokartan hakuun"""
    return jsonify({'metromap': current_metromap, 'timestamp': datetime.now().strftime('%H:%M:%S')})

@app.route('/api/node-details/<node_id>')
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

@app.route('/api/search/<search_term>')
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
                
                if is_timestamp_search:
                    entry_time = entry.get('formatted_time', '')
                    if clean_search_term in entry_time:
                        match_found = True
                elif is_combined_search:
                    entry_time = entry.get('formatted_time', '')
                    entry_value = entry.get('value', '').lower()
                    if text_part in entry_value and time_part in entry_time:
                        match_found = True
                else:
                    if search_lower in entry.get('value', '').lower():
                        match_found = True
                
                if match_found:
                    if is_timestamp_search:
                        if '.' in clean_search_term and ':' in clean_search_term:
                            display_text = f"{details.get('type', 'Unknown')}: {details.get('value', 'Unknown')} (Full timestamp match: {entry_time})"
                        elif '.' in clean_search_term and ':' not in clean_search_term:
                            display_text = f"{details.get('type', 'Unknown')}: {details.get('value', 'Unknown')} (Date match: {entry_time})"
                        elif ':' in clean_search_term and '.' not in clean_search_term:
                            display_text = f"{details.get('type', 'Unknown')}: {details.get('value', 'Unknown')} (Time match: {entry_time})"
                    elif is_combined_search:
                        display_text = f"{details.get('type', 'Unknown')}: {details.get('value', 'Unknown')} (Combined match: {entry_value} at {entry_time})"
                    else:
                        display_text = f"{details.get('type', 'Unknown')}: {details.get('value', 'Unknown')} (part of group)"
                    
                    results.append({
                        'node_id': node_id,
                        'display_text': display_text,
                        'match_type': 'entry'
                    })
                    found_in_entries = True
                    break

        if not found_in_entries and 'technical_entries' in details:
            for entry in details['technical_entries']:
                match_found = False
                
                if is_timestamp_search:
                    entry_time = entry.get('formatted_time', '')
                    if clean_search_term in entry_time:
                        match_found = True
                elif is_combined_search:
                    entry_time = entry.get('formatted_time', '')
                    entry_value = entry.get('value', '').lower()
                    if text_part in entry_value and time_part in entry_time:
                        match_found = True
                else:
                    if search_lower in entry.get('value', '').lower():
                        match_found = True
                
                if match_found:
                    if is_timestamp_search:
                        if '.' in clean_search_term and ':' in clean_search_term:
                            display_text = f"{details.get('type', 'Unknown')}: {details.get('value', 'Unknown')} (Full timestamp match: {entry_time} (technical entry))"
                        elif '.' in clean_search_term and ':' not in clean_search_term:
                            display_text = f"{details.get('type', 'Unknown')}: {details.get('value', 'Unknown')} (Date match: {entry_time} (technical entry))"
                        elif ':' in clean_search_term and '.' not in clean_search_term:
                            display_text = f"{details.get('type', 'Unknown')}: {details.get('value', 'Unknown')} (Time match: {entry_time} (technical entry))"
                    elif is_combined_search:
                        display_text = f"{details.get('type', 'Unknown')}: {details.get('value', 'Unknown')} (Combined match: {text_part} at {time_part} in technical)"
                    else:
                        display_text = f"{details.get('type', 'Unknown')}: {details.get('value', 'Unknown')} (found in technical entries)"
                    
                    results.append({
                        'node_id': node_id,
                        'display_text': display_text,
                        'match_type': 'technical'
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

def create_html_file(metromap_content):
    """Luo staattinen HTML-tiedosto"""
    html_content = f"""<!DOCTYPE html>
<html>
<head>
    <title>Mermetro</title>
    <script type="module">
        import mermaid from 'https://cdn.jsdelivr.net/npm/mermaid@10/dist/mermaid.esm.min.mjs';
        mermaid.initialize({{ startOnLoad: true }});
    </script>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; background-color: #f5f5f5; }}
        .mermaid {{ text-align: center; background-color: white; padding: 20px; border-radius: 10px; }}
    </style>
</head>
<body>
    <h1>Mermetro</h1>
    <div class="mermaid">
{metromap_content}
    </div>
</body>
</html>"""
    
    with open('metrokartta.html', 'w', encoding='utf-8') as f:
        f.write(html_content)

def start_file_watcher():
    """Käynnistä tiedoston seuranta"""
    event_handler = JSONFileHandler()
    observer = Observer()
    observer.schedule(event_handler, path='.', recursive=False)
    observer.start()
    return observer

def main():
    print("\nStarting up...")
    print(f"Time: {datetime.now().strftime('%d.%m.%Y %H:%M:%S')}")
    
    find_json()
    process_json_file()
    observer = start_file_watcher()

    with open('metrokartta_koodi.txt', 'w', encoding='utf-8') as f:
        f.write(current_metromap)
    create_html_file(current_metromap)
    
    print("\nCreated files:")
    print("   metrokartta_koodi.txt  -> Mermaid-code")
    print("   metrokartta.html       -> Static HTML")
    print("\nAccess:")
    print("   Live-page: http://localhost:5000")
    print("   Static file: metrokartta.html\n")
    
    try:
        app.run(debug=False, host='127.0.0.1', port=5000)
    except KeyboardInterrupt:
        print("\nStopped by user")
    finally:
        observer.stop()
        observer.join()

if __name__ == "__main__":
    main()
