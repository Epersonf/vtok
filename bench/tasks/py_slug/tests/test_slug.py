from slug import slugify

def test_basic():
    assert slugify("Hello, World!") == "hello-world"
    assert slugify("  --A  b__c-- ") == "a-b-c"

def test_max_len():
    assert slugify("hello world foo", max_len=8) == "hello-wo"
    assert slugify("hello world", max_len=6) == "hello"
    assert slugify("abc", max_len=10) == "abc"
