import pytest
from account import Account, InsufficientFunds

def test_ops_and_history():
    a = Account(10); a.deposit(5); a.withdraw(3)
    assert a.balance == 12 and a.history == [("deposit", 5), ("withdraw", 3)]

def test_errors_leave_state():
    a = Account(5)
    with pytest.raises(InsufficientFunds):
        a.withdraw(6)
    for f in (a.deposit, a.withdraw):
        with pytest.raises(ValueError):
            f(0)
    assert a.balance == 5 and a.history == []

def test_transfer():
    a, b = Account(10), Account(1)
    a.transfer(b, 4)
    assert (a.balance, b.balance) == (6, 5)
    with pytest.raises(InsufficientFunds):
        a.transfer(b, 100)
    assert (a.balance, b.balance) == (6, 5)
