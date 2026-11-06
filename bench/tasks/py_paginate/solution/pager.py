def paginate(items, page, size):
    if page < 1 or size < 1:
        raise ValueError("page and size must be >= 1")
    start = (page - 1) * size
    return list(items[start:start + size])


def page_count(total, size):
    return (total + size - 1) // size
