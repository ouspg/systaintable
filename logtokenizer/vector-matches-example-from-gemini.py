import numpy as np
from itertools import combinations

# Example token vectors (log lines)
# Vector length (L) is 6.
# 0 and 2 differ at index 1 and 3 (Distance = 2)
# 0 and 1 differ at index 2 and 5 (Distance = 2)
# 1 and 5 are identical (Distance = 0)
# 0 and 4 are identical (Distance = 0)
# 1 and 6 differ at index 2 only (Distance = 1) <--- The target pattern
log_vectors = [
    [101, 205, 301, 410, 500, 602], # Index 0
    [101, 205, 302, 410, 500, 603], # Index 1
    [101, 206, 301, 411, 500, 602], # Index 2
    [700, 800, 900, 100, 110, 120], # Index 3 (Very different)
    [101, 205, 301, 410, 500, 602], # Index 4 (Duplicate of 0)
    [101, 205, 302, 410, 500, 603], # Index 5 (Duplicate of 1)
    [101, 205, 305, 410, 500, 603]  # Index 6 (Differs from 1 at index 2: 305 vs 302)
]

# Convert to NumPy array
vectors = np.array(log_vectors)

def calculate_hamming_distance(vector_a, vector_b):
    """
    Calculates the Hamming Distance: the number of positions at which the
    corresponding elements are different.
    """
    # The '==' operator returns a boolean array (True for match, False for difference).
    # Subtracting this from 1 (or taking the negation ~) yields a 1 for difference, 0 for match.
    # Summing this result gives the Hamming Distance.
    return np.sum(vector_a != vector_b)

def find_single_difference_pairs(all_vectors):
    """
    Finds all unique pairs of vectors (indices) that have a Hamming Distance of exactly 1.
    """
    # Use combinations to check every unique pair only once
    num_vectors = len(all_vectors)
    single_diff_pairs = []

    # Iterate through all unique pairs of indices (i, j)
    for i, j in combinations(range(num_vectors), 2):
        vector_i = all_vectors[i]
        vector_j = all_vectors[j]
        
        # NOTE: Handle identical vectors (Distance 0) if you don't want to include them
        # in the count of (i, j) pairs. The 'combinations' handles the (i, i) case.

        distance = calculate_hamming_distance(vector_i, vector_j)

        if distance == 1:
            # Found a pair that differs in exactly one position
            single_diff_pairs.append((i, j))
            
    return single_diff_pairs

# --- Execution ---
single_change_pairs = find_single_difference_pairs(vectors)

print("Pairs of vectors (indices) that differ in exactly one position (Hamming Distance = 1):")
print("---")
if single_change_pairs:
    for i, j in single_change_pairs:
        # Find the index where the difference occurs
        diff_position = np.where(vectors[i] != vectors[j])[0][0]
        
        print(f"Indices: ({i}, {j})")
        print(f"  Difference at position (index): {diff_position}")
        print(f"  Vector {i}: {vectors[i].tolist()}")
        print(f"  Vector {j}: {vectors[j].tolist()}")
        print("-" * 20)
else:
    print("No pairs found with a Hamming Distance of exactly 1.")