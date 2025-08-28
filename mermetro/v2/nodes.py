import re
import string

def get_node_value(node_key, node_details):
    """Hakee noden arvon node_details"""
    if node_key in node_details:
        member_value = node_details[node_key].get('value', node_key)
        if node_details[node_key].get('type') == 'URL' and '?' in member_value:
            return member_value.split('?')[0]
        return member_value
    return node_key

def clean_value_for_mermaid(value):
    """Puhdistaa arvon Mermaid-kaaviota varten"""
    clean_value = str(value)
    
    ALLOWED_CHARS = string.ascii_letters + string.digits + '_'
    clean_value = re.sub('_+', '_', re.sub(f"[^{re.escape(ALLOWED_CHARS)}]", '_', clean_value))
    
    clean_value = clean_value.strip('_')
    
    if not clean_value:
        clean_value = "empty"
    
    if len(clean_value) > 20:
        clean_value = clean_value[:20]
    
    if clean_value and not clean_value[0].isalpha():
        clean_value = "n" + clean_value
    
    return clean_value

def generate_nodes_content(group_id, node_details):
    """Generoi class diagram"""
    if group_id not in node_details:
        return "classDiagram\n    class ERROR {\n        +Group not found\n    }"
    
    group_data = node_details[group_id]
    group_entries = group_data.get('entries', [])
    
    if not group_entries:
        return "classDiagram\n    class ERROR {\n        +No entries found for this group\n    }"
    
    entries_by_value = {}
    
    for entry in group_entries:
        value = entry.get('value', '')
        entry_type = entry.get('type', 'Unknown')
        line = entry.get('line', None)
        
        if value not in entries_by_value:
            entries_by_value[value] = {
                'type': entry_type,
                'lines': set(),
                'count': 0
            }
        
        entries_by_value[value]['lines'].add(line)
        entries_by_value[value]['count'] += 1
    
    lines_to_entries = {}
    lines_to_timestamps = {}
    
    for entry in group_entries:
        line = entry.get('line', None)
        value = entry.get('value', '')
        timestamp = entry.get('timestamp', None)
        
        if line is not None:
            if line not in lines_to_entries:
                lines_to_entries[line] = set()
            lines_to_entries[line].add(value)
            
            if line not in lines_to_timestamps and timestamp:
                lines_to_timestamps[line] = timestamp
    
    all_entries = set(entries_by_value.keys())
    covered_entries = set()
    selected_lines = []
    
    # algoritmi valitsee rivit, jotka kattavat eniten vielä kattamattomia entryjä
    while covered_entries != all_entries:
        best_line = None
        best_new_coverage = 0
        
        for line, line_entries in lines_to_entries.items():
            if line in selected_lines:
                continue
                
            new_coverage = len(line_entries - covered_entries)
            
            if new_coverage > best_new_coverage:
                best_new_coverage = new_coverage
                best_line = line
        
        if best_line is not None:
            selected_lines.append(best_line)
            covered_entries.update(lines_to_entries[best_line])
        else:
            remaining_lines = [line for line in lines_to_entries.keys() if line not in selected_lines]
            if remaining_lines:
                selected_lines.append(remaining_lines[0])
                covered_entries.update(lines_to_entries[remaining_lines[0]])
            else:
                break
    
    content = "---\n"
    content += "    config:\n"
    content += "        class:\n"
    content += "            hideEmptyMembersBox: true\n"
    content += "---\n"
    content += "classDiagram\n\n"

    line_to_class_name = {}
    line_class_order = []

    for line in sorted(selected_lines):
        class_name = f"LINE_{line}"
        line_to_class_name[line] = class_name
        line_class_order.append(class_name)
        
        timestamp = lines_to_timestamps.get(line, "N/A")
    
        content += f"class {class_name} {{\n"

        line_entries = sorted(list(lines_to_entries[line]))
        for entry_value in line_entries:
            clean_value = str(entry_value)
            clean_value = clean_value.replace('"', "'")
            clean_value = clean_value.replace('\n', ' ')
            clean_value = clean_value.replace('[', '(')
            clean_value = clean_value.replace(']', ')')
            clean_value = clean_value.replace('{', '(')
            clean_value = clean_value.replace('}', ')')
            clean_value = clean_value.replace('|', '-')
            clean_value = clean_value.replace('<', '(')
            clean_value = clean_value.replace('>', ')')
            if len(clean_value) > 30:
                clean_value = clean_value[:27] + "..."
            content += f"    {clean_value}\n"
        content += f"    Time: ({timestamp})\n"
        content += "}\n\n"

    if len(line_class_order) > 5:
        for i in range(5, len(line_class_order)):
            prev_class = line_class_order[i - 5]
            curr_class = line_class_order[i]
            content += f"{prev_class} .. {curr_class}\n"

    processed_connections = set()
    merge_logs = group_data.get('merge_log', [])
    for log_entry in merge_logs:
        if "(" in log_entry and ")" in log_entry:
            start_idx = log_entry.find("(") + 1
            end_idx = log_entry.find(")")
            if start_idx > 0 and end_idx > start_idx:
                tuple_content = log_entry[start_idx:end_idx]
                values = [v.strip() for v in tuple_content.split(",")]
                if len(values) >= 2:
                    val1, val2 = values[0], values[1]
                    lines_for_val1 = []
                    lines_for_val2 = []
                    for line in selected_lines:
                        if val1 in lines_to_entries[line]:
                            lines_for_val1.append(line)
                        if val2 in lines_to_entries[line]:
                            lines_for_val2.append(line)
                    for line1 in lines_for_val1:
                        for line2 in lines_for_val2:
                            if line1 != line2:
                                connection_key = tuple(sorted([line1, line2]))
                                if connection_key not in processed_connections:
                                    class1 = line_to_class_name[line1]
                                    class2 = line_to_class_name[line2]
                                    processed_connections.add(connection_key)

    return content

def get_node_relationships(group_id, node_details):
    """Palauttaa yksityiskohtaiset tiedot ryhmän nodeista"""
    if group_id not in node_details:
        return {}
    
    group_data = node_details[group_id]
    group_entries = group_data.get('entries', [])
    
    relationships = {
        'line_connections': {},
        'tuple_connections': [],  
        'entry_details': {} 
    }
    
    for entry in group_entries:
        value = entry.get('value', '')
        relationships['entry_details'][value] = entry
        
        line = entry.get('line', None)
        if line is not None:
            if line not in relationships['line_connections']:
                relationships['line_connections'][line] = []
            if value not in relationships['line_connections'][line]:
                relationships['line_connections'][line].append(value)
    
    merge_logs = group_data.get('merge_log', [])
    for log_entry in merge_logs:
        if "(" in log_entry and ")" in log_entry:
            start_idx = log_entry.find("(") + 1
            end_idx = log_entry.find(")")
            if start_idx > 0 and end_idx > start_idx:
                tuple_content = log_entry[start_idx:end_idx]
                values = [v.strip() for v in tuple_content.split(",")]
                if len(values) >= 2:
                    val1, val2 = values[0], values[1]
                    context = "FORMED" if "FORMED:" in log_entry else "ADDED" if "ADDED:" in log_entry else "MERGED"
                    relationships['tuple_connections'].append((val1, val2, context))
    
    return relationships