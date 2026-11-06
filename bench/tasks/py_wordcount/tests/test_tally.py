from tally import word_counts, top_n

def test_normalizes():
    assert word_counts("The cat, the CAT! (dog)") == {"the": 2, "cat": 2, "dog": 1}

def test_empty_words_dropped():
    assert word_counts("-- ... a") == {"a": 1}

def test_top_n():
    t = "b a b a c c c"
    assert top_n(t, 2) == [("c", 3), ("a", 2)]
    assert top_n(t, 10) == [("c", 3), ("a", 2), ("b", 2)]
    assert top_n(t, 0) == [] and top_n(t, -1) == []
