"""Word statistics."""

def word_counts(text):
    """Count words in text."""
    counts = {}
    for word in text.split():
        counts[word] = counts.get(word, 0) + 1
    return counts
