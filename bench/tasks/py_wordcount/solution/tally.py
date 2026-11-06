import string


def word_counts(text):
    counts = {}
    for word in text.split():
        word = word.lower().strip(string.punctuation)
        if not word:
            continue
        counts[word] = counts.get(word, 0) + 1
    return counts


def top_n(text, n):
    if n <= 0:
        return []
    items = sorted(word_counts(text).items(), key=lambda kv: (-kv[1], kv[0]))
    return items[:n]
