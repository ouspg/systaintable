import numpy as np
from itertools import combinations
import copy # Used for deep copying the list of vectors

# --- Setup ---
log_vectors = [
    [101, 205, 301, 410, 500, 602], # Pattern A
    [101, 205, 302, 410, 500, 603], # Pattern B
    [101, 206, 301, 411, 500, 602], # Pattern C (Differs from A by 2)
    [101, 205, 305, 410, 500, 603], # Pattern B' (Differs from B by 1)
    [101, 205, 301, 410, 500, 602], # Pattern A (Duplicate)
    [700, 800, 900, 100, 110, 120], # Pattern D (Unique)
    [101, 205, 301, 410, 500, 603], # Pattern B'' (Differs from A by 1, from B by 1)
    [101, 205, 302, 410, 500, 603], # Pattern B (Duplicate)
]

vectors = np.array(log_vectors)
MAGIC_TOKEN = 0 # Placeholder for variable tokens

# ----------------------------------------------------
# --- Core Helper Functions (Unchanged) ---
# ----------------------------------------------------

def find_pairs_by_hamming_distance(all_vectors, target_distance):
    """Finds all unique pairs that have the exact target Hamming Distance."""
    num_vectors = len(all_vectors)
    matching_pairs = []
    
    # Use combinations to check every unique pair (i, j)
    for i, j in combinations(range(num_vectors), 2):
        vector_i = all_vectors[i]
        vector_j = all_vectors[j]
        
        distance = np.sum(vector_i != vector_j)

        if distance == target_distance:
            diff_positions = np.where(vector_i != vector_j)[0].tolist()
            
            matching_pairs.append({
                'indices': (i, j), 
                'diff_positions': diff_positions,
                'distance': distance
            })
            
    return matching_pairs

def create_pattern_rule(vector_a, diff_positions, magic_token):
    """Creates a rule vector by replacing variable positions with the magic token."""
    rule_vector = np.copy(vector_a)
    
    # Replace tokens at ALL difference positions
    for pos in diff_positions:
        rule_vector[pos] = magic_token
    
    return rule_vector

def match_vector_to_rule(vector, rule_vector, magic_token):
    """Checks if a vector matches the rule."""
    for v_token, r_token in zip(vector, rule_vector):
        # Mismatch if the rule requires a fixed token (r_token != magic_token) 
        # but the vector has a different value (v_token != r_token)
        if r_token != magic_token and v_token != r_token:
            return False
    return True

# ----------------------------------------------------
# --- Main Iterative Function ---
# ----------------------------------------------------

def iterate_rule_creation(original_vectors, target_distance):
    """
    Iteratively finds patterns, creates rules, and removes matched vectors.
    """
    # Create a deep copy to modify the list without changing the original data
    remaining_vectors = copy.deepcopy(original_vectors)
    all_rules = []
    rule_id = 1
    
    print(f"Starting iteration with {len(remaining_vectors)} vectors and Target Distance = {target_distance}")

    while True:
        # 1. Find pairs in the remaining vectors
        pairs = find_pairs_by_hamming_distance(remaining_vectors, target_distance)

        if not pairs:
            print("\n--- ITERATION ENDED ---")
            print(f"No more pairs found with Hamming Distance {target_distance}.")
            break # Exit the loop if no more pairs are found

        # 2. Select the first pair found and its vector
        first_pair_info = pairs[0]
        i, j = first_pair_info['indices']
        diff_positions = first_pair_info['diff_positions']
        
        # We use the vector at index 'i' in the *remaining_vectors* list
        seed_vector = remaining_vectors[i] 
        
        # 3. Create the pattern rule
        new_rule = create_pattern_rule(seed_vector, diff_positions, MAGIC_TOKEN)
        
        # 4. Match the rule against all remaining vectors
        matched_indices = []
        vectors_to_keep = []
        matched_vectors_for_rule = []
        
        # We need to iterate backward or use a temporary list to handle removal
        # We'll use a new list (vectors_to_keep) for the non-matched vectors.
        for k, current_vector in enumerate(remaining_vectors):
            if match_vector_to_rule(current_vector, new_rule, MAGIC_TOKEN):
                matched_vectors_for_rule.append(current_vector.tolist())
                matched_indices.append(k) # Track original index for debugging, not crucial for final logic
            else:
                vectors_to_keep.append(current_vector)

        # 5. Store the results and update the remaining vectors list
        all_rules.append({
            'rule_id': rule_id,
            'template': new_rule.tolist(),
            'match_count': len(matched_vectors_for_rule),
            'matched_vectors': matched_vectors_for_rule
        })

        # Update the list for the next iteration
        vectors_removed = len(remaining_vectors) - len(vectors_to_keep)
        remaining_vectors = vectors_to_keep
        
        print(f"\nRule {rule_id} generated. Removed {vectors_removed} vectors.")
        print(f"  Template: {new_rule.tolist()}")
        print(f"  Remaining vectors: {len(remaining_vectors)}")

        rule_id += 1
        
    # After the loop, the remaining vectors are those that didn't form a pattern
    # at the specified Hamming distance.
    return all_rules, remaining_vectors


# --- Execution ---
TARGET_DISTANCE = 2 # Example: Group vectors that differ by exactly 2 positions

rules, remaining = iterate_rule_creation(vectors, TARGET_DISTANCE)

print("\n\n--- FINAL RESULTS ---")
print(f"Total rules generated: {len(rules)}")
for rule in rules:
    print(f"\n## 📜 Rule {rule['rule_id']} (Matches: {rule['match_count']})")
    print(f"Template: {rule['template']}")
    # print(f"Examples: {rule['matched_vectors'][:2]}") # Print first 2 examples

if remaining:
    print(f"\n## 🗃️ Unclustered Vectors ({len(remaining)})")
    # Convert remaining NumPy arrays back to lists for clean output
    print([v.tolist() for v in remaining])