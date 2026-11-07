"""Task `py_account` (python, target account.py): the instruction, the project to change, its tests and a reference solution."""
TASK = dict(lang="python", target="account.py", cmd="py", instruction=
 "Add to `Account`: class `InsufficientFunds(Exception)`; `withdraw` must raise it (instead of going negative) when amount > balance, "
 "and raise ValueError for amount <= 0 (same for `deposit`); a `history` list of ('deposit'|'withdraw', amount) tuples recorded for "
 "successful operations; and `transfer(other, amount)` that withdraws from self and deposits to other (nothing changes on failure).",
 src={"account.py": '''"""Bank account."""

class Account:
    def __init__(self, balance=0):
        self.balance = balance

    def deposit(self, amount):
        self.balance += amount

    def withdraw(self, amount):
        self.balance -= amount
'''},
 tests={"test_account.py": '''import pytest
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
'''},
 solution={"account.py": '''class InsufficientFunds(Exception):
    pass


class Account:
    def __init__(self, balance=0):
        self.balance = balance
        self.history = []

    def deposit(self, amount):
        if amount <= 0:
            raise ValueError("amount must be positive")
        self.balance += amount
        self.history.append(("deposit", amount))

    def withdraw(self, amount):
        if amount <= 0:
            raise ValueError("amount must be positive")
        if amount > self.balance:
            raise InsufficientFunds()
        self.balance -= amount
        self.history.append(("withdraw", amount))

    def transfer(self, other, amount):
        self.withdraw(amount)
        other.deposit(amount)
'''})
