def get_node_value(node_key, node_details):
    """Hakee noden arvon node_details"""
    if node_key in node_details:
        member_value = node_details[node_key].get('value', node_key)
        if node_details[node_key].get('type') == 'URL' and '?' in member_value:
            return member_value.split('?')[0]
        return member_value
    return node_key

def group_by_person(connections, node_details):
    """Luodaan henkilöryhmiä yhteyksien perusteella"""
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
                
                group_a_members = [get_node_value(member.split('(')[0], node_details) for member in group_a]
                group_b_members = [get_node_value(member.split('(')[0], node_details) for member in group_b]
                
                group_a.update(group_b)
                
                a_logs = temp_merge_logs.get(group_a_idx, [])
                b_logs = temp_merge_logs.get(group_b_idx, [])
                
                group_a_str = ", ".join(sorted(group_a_members))
                group_b_str = ", ".join(sorted(group_b_members))
                
                merged_log = f"MERGED: Group A [{group_a_str}]\n + \nGroup B [{group_b_str}] \nbecause of Tuple ({get_node_value(a.split('(')[0], node_details)} , {get_node_value(b.split('(')[0], node_details)})"
                temp_merge_logs[group_a_idx] = a_logs + b_logs + [merged_log]
                
                del person_groups[group_b_idx]
                if group_b_idx in temp_merge_logs:
                    del temp_merge_logs[group_b_idx]
                    
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
                
            a_value = get_node_value(a.split('(')[0], node_details)
            b_value = get_node_value(b.split('(')[0], node_details)
            temp_merge_logs[idx].append(f"ADDED: ({a_value} , {b_value}) -> added to group")
            
        elif groups_with_b:
            # Lisää a ryhmään B
            idx = groups_with_b[0]
            person_groups[idx].add(a)
            
            if idx not in temp_merge_logs:
                temp_merge_logs[idx] = []
                
            a_value = get_node_value(a.split('(')[0], node_details)
            b_value = get_node_value(b.split('(')[0], node_details)
            temp_merge_logs[idx].append(f"ADDED: ({a_value} , {b_value}) -> added to group")
            
        else:
            # Luo uusi ryhmä
            person_groups.append({a, b})
            current_idx = len(person_groups) - 1
            
            a_value = get_node_value(a.split('(')[0], node_details)
            b_value = get_node_value(b.split('(')[0], node_details)
            temp_merge_logs[current_idx] = [f"FORMED: ({a_value} , {b_value}) = new group"]

    group_merge_log = {}
    for i, group in enumerate(person_groups):
        final_group_id = f"ID_{i + 1}"
        group_merge_log[final_group_id] = temp_merge_logs.get(i, [])

    return person_groups, group_merge_log

def create_formed_from_data(group_id, val1, val2, node_details):
    group_entries = node_details[group_id].get('entries', []) if group_id in node_details else []
    
    line_to_values = {}
    line_to_entries = {}
    for entry in group_entries:
        line = entry.get('line', None)
        if line is not None:
            if line not in line_to_values:
                line_to_values[line] = set()
                line_to_entries[line] = []
            line_to_values[line].add(entry.get('value', ''))
            line_to_entries[line].append(entry)
    
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
            for entry in line_to_entries[found_line]:
                if entry.get('value') == v:
                    formed_from.append({
                        'value': v,
                        'line': found_line,
                        'timestamp': entry.get('timestamp', 'N/A'),
                        'type': entry.get('type', 'Unknown'),
                        'tuple_line': found_line
                    })
                    break
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

def generate_timeline_content(group_id, node_details):
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

                formed_from = create_formed_from_data(group_id, val1, val2, node_details)
                
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

                formed_from = create_formed_from_data(group_id, val1, val2, node_details)
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

            merging_tuple = []
            original_tuple_values = [v.strip() for v in log_entry[tuple_start:tuple_end].split(",") if v.strip()]
            if len(original_tuple_values) >= 2:
                val1, val2 = original_tuple_values[0], original_tuple_values[1]
                merging_tuple = create_formed_from_data(group_id, val1, val2, node_details)
            else:
                merging_tuple = []
            
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
