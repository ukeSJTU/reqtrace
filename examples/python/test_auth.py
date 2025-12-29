"""
Authentication service tests

This module tests the account locking mechanism for brute-force protection.
"""

from datetime import datetime, timedelta

import pytest


class TestAccountLocking:
    """
    Test account locking logic
    @reqtrace:REQ-01
    """

    def test_account_lock_after_failed_attempts(self):
        """
        Test that account locks after 5 failed login attempts
        @reqtrace:REQ-01.AC-01
        """
        # Given: account is in normal state
        user_id = "test_user_123"
        login_service = LoginService()
        
        # When: user enters wrong password 5 times in 5 minutes
        for i in range(5):
            result = login_service.attempt_login(user_id, "wrong_password")
            assert result.success is False
        
        # Then: account should be locked for 30 minutes
        assert login_service.is_locked(user_id) is True
        lock_duration = login_service.get_lock_duration(user_id)
        assert lock_duration.total_seconds() == 30 * 60
        
        # And: security warning email should be sent
        assert login_service.email_sent(user_id, "security_warning") is True

    def test_login_rejected_when_locked(self):
        """
        Test login rejection during lock period
        @reqtrace:REQ-01.AC-02
        """
        # Given: account is already locked
        user_id = "locked_user_456"
        login_service = LoginService()
        login_service.lock_account(user_id, duration_minutes=30)
        
        # When: user enters correct password
        result = login_service.attempt_login(user_id, "correct_password")
        
        # Then: login should be rejected
        assert result.success is False
        assert "账户锁定中" in result.message
        
    def test_partial_lock_scenario(self):
        """
        Test account lock with only 3 failed attempts (should not lock)
        @reqtrace:REQ-01.AC-01
        """
        user_id = "test_user_789"
        login_service = LoginService()
        
        # Only 3 failed attempts - should not trigger lock
        for i in range(3):
            login_service.attempt_login(user_id, "wrong_password")
        
        assert login_service.is_locked(user_id) is False


class LoginService:
    """Mock login service for testing"""
    
    def __init__(self):
        self.attempts = {}
        self.locked = {}
        self.emails = {}
    
    def attempt_login(self, user_id, password):
        if self.is_locked(user_id):
            return LoginResult(False, "账户锁定中")
        
        # Mock password check
        if password != "correct_password":
            self.attempts[user_id] = self.attempts.get(user_id, 0) + 1
            if self.attempts[user_id] >= 5:
                self.lock_account(user_id, 30)
                self.send_email(user_id, "security_warning")
            return LoginResult(False, "密码错误")
        
        return LoginResult(True, "登录成功")
    
    def is_locked(self, user_id):
        if user_id not in self.locked:
            return False
        lock_until = self.locked[user_id]
        return datetime.now() < lock_until
    
    def lock_account(self, user_id, duration_minutes):
        self.locked[user_id] = datetime.now() + timedelta(minutes=duration_minutes)
    
    def get_lock_duration(self, user_id):
        if user_id in self.locked:
            return self.locked[user_id] - datetime.now()
        return timedelta(0)
    
    def send_email(self, user_id, email_type):
        if user_id not in self.emails:
            self.emails[user_id] = []
        self.emails[user_id].append(email_type)
    
    def email_sent(self, user_id, email_type):
        return user_id in self.emails and email_type in self.emails[user_id]


class LoginResult:
    def __init__(self, success, message):
        self.success = success
        self.message = message
