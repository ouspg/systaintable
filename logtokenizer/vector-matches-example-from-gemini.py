import numpy as np

# Example token vectors (log lines)
# Each number represents a token ID. Vectors must be the same length for this approach.
log_vectors = [
    [101, 205, 301, 410, 500, 602],
    [101, 205, 302, 410, 500, 603],
    [101, 206, 301, 411, 500, 602],
    [700, 800, 900, 100, 110, 120],
    [101, 205, 301, 410, 500, 602] # Duplicate of vector 0
]

# Convert to NumPy array for efficient comparison
vectors = np.array(log_vectors)

def positional_overlap_similarity(vector_a, vector_b):
    """
    Calculates the number of tokens that are identical and in the same position
    between two vectors. (Assuming same-length vectors).
    """
    # Uses boolean comparison and sums the 'True' values (which are 1)
    return np.sum(vector_a == vector_b)

def find_most_similar_vectors(target_vector, all_vectors, top_n=3):
    """
    Compares a target vector to all other vectors and returns the top N
    most similar by positional overlap.
    """
    similarity_scores = []
    
    # Iterate through all vectors, excluding the target vector itself (if it's in the list)
    for i, current_vector in enumerate(all_vectors):
        # Calculate similarity
        score = positional_overlap_similarity(target_vector, current_vector)
        
        # Store index (for identification) and the score
        similarity_scores.append((i, score, current_vector.tolist()))

    # Sort the list by score in descending order
    # The key is a lambda function to sort by the score (index 1)
    similarity_scores.sort(key=lambda x: x[1], reverse=True)
    
    # Return the top N results
    return similarity_scores[:top_n]

# --- Example Usage ---
# Use the first vector as the target for comparison
target_vector = vectors[0]

# Find the top 3 vectors most similar to the target_vector
top_similar = find_most_similar_vectors(target_vector, vectors, top_n=3)

print(f"Target Vector (Index 0): {target_vector.tolist()}")
print("\nTop Similar Vectors by Positional Overlap:")
for index, score, vector in top_similar:
    print(f"Index: {index}, Score (Matches): {score}, Vector: {vector}")