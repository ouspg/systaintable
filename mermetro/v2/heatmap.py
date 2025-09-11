from . import mermetro2

def generate_heatmap_segments(timeline_data, segments=200):
    if not timeline_data:
        return []
    
    min_time = timeline_data['min_timestamp']
    max_time = timeline_data['max_timestamp']
    duration = max_time - min_time
    
    if duration.total_seconds() < 3600:
        segments = 60
    elif duration.total_seconds() < 24 * 3600:
        segments = 120
    elif duration.days < 7:
        segments = 168
    
    segment_duration = duration / segments
    timestamps = timeline_data['timestamps']
    type_distribution = timeline_data.get('type_distribution', {})
    
    most_common_types = sorted(type_distribution.items(), key=lambda x: x[1], reverse=True)[:5]
    dominant_type = most_common_types[0][0] if most_common_types else 'Unknown'
    
    segments_data = []
    
    for i in range(segments):
        segment_start = min_time + (segment_duration * i)
        segment_end = min_time + (segment_duration * (i + 1))
        
        segment_timestamps = [ts for ts in timestamps if segment_start <= ts < segment_end]
        count = len(segment_timestamps)
        
        if count == 0:
            activity_level = 'none'
        elif count <= 2:
            activity_level = 'low'
        elif count <= 10:
            activity_level = 'medium'
        elif count <= 50:
            activity_level = 'high'
        else:
            activity_level = 'extreme'
        
        segments_data.append({
            'start': segment_start,
            'end': segment_end,
            'count': count,
            'percentage': (i / segments) * 100,
            'dominant_type': dominant_type,
            'activity_level': activity_level
        })
    
    return segments_data

def get_heatmap_statistics(timeline_data):
    if not timeline_data:
        return {}
    
    hourly_activity = timeline_data.get('hourly_activity', {})
    daily_activity = timeline_data.get('daily_activity', {})
    value_distribution = timeline_data.get('value_distribution', {})
    
    busiest_hours = sorted(hourly_activity.items(), key=lambda x: x[1], reverse=True)[:3]
    busiest_days = sorted(daily_activity.items(), key=lambda x: x[1], reverse=True)[:3]
    top_values = sorted(value_distribution.items(), key=lambda x: x[1], reverse=True)[:5]
    
    avg_hourly = sum(hourly_activity.values()) / len(hourly_activity) if hourly_activity else 0
    avg_daily = sum(daily_activity.values()) / len(daily_activity) if daily_activity else 0
    unique_entries = timeline_data.get('unique_entries', 0)
    
    return {
        'busiest_hours': [
            {
                'time': hour.strftime('%Y-%m-%d %H:00'),
                'count': count,
                'formatted': hour.strftime('%H:00 on %b %d')
            }
            for hour, count in busiest_hours
        ],
        'busiest_days': [
            {
                'date': day.strftime('%Y-%m-%d'),
                'count': count,
                'formatted': day.strftime('%B %d, %Y')
            }
            for day, count in busiest_days
        ],
        'top_types': [
            {
                'type': entry_value, 
                'count': count, 
                'percentage': round((count / timeline_data['total_entries']) * 100, 1)
            }
            for entry_value, count in top_values
        ],
        'avg_hourly': round(avg_hourly, 1),
        'avg_daily': round(avg_daily, 1),
        'unique_entries': unique_entries,
        'total_active_hours': len(hourly_activity),
        'total_active_days': len(daily_activity)
    }

def analyze_group_timeline_heatmap(group_id, node_details):
    if group_id not in node_details:
        return None
        
    group_data = node_details[group_id]
    group_entries = group_data.get('entries', [])
    
    if not group_entries:
        return None
    
    timestamps = []
    hourly_activity = {}
    daily_activity = {}
    type_distribution = {}
    value_distribution = {}
    unique_node_ids = set()

    for entry in group_entries:
        timestamp_str = entry.get('timestamp', 'N/A')
        if timestamp_str == 'N/A':
            continue
            
        dt = mermetro2.parse_timestamp_to_datetime(timestamp_str)
        if dt:
            timestamps.append(dt)
            
            hour_key = dt.replace(minute=0, second=0)
            hourly_activity[hour_key] = hourly_activity.get(hour_key, 0) + 1
            
            day_key = dt.replace(hour=0, minute=0, second=0)
            daily_activity[day_key] = daily_activity.get(day_key, 0) + 1
            
            entry_type = entry.get('type', 'Unknown')
            type_distribution[entry_type] = type_distribution.get(entry_type, 0) + 1
            
            entry_value = entry.get('value', 'Unknown')
            value_distribution[entry_value] = value_distribution.get(entry_value, 0) + 1
            
            try:
                node_identities = mermetro2.parse_identities(entry)
                for node_id in node_identities:
                    if '(' in node_id:
                        clean_node_id = node_id.split('(')[0]
                        unique_node_ids.add(clean_node_id)
            except Exception:
                clean_value = mermetro2.UNDERSCORE_REGEX.sub('_', mermetro2.CLEAN_REGEX.sub('_', entry_value))
                fallback_node_id = f"{entry_type}_{clean_value}"
                unique_node_ids.add(fallback_node_id)
    
    if not timestamps:
        return None
        
    timestamps.sort()
    min_time = timestamps[0]
    max_time = timestamps[-1]
    total_duration = max_time - min_time
    
    return {
        'min_timestamp': min_time,
        'max_timestamp': max_time,
        'total_entries': len(timestamps),
        'unique_entries': len(unique_node_ids),
        'duration_days': total_duration.days,
        'duration_hours': total_duration.total_seconds() / 3600,
        'hourly_activity': hourly_activity,
        'daily_activity': daily_activity,
        'value_distribution': value_distribution,
        'timestamps': timestamps,
        'group_id': group_id
    }
