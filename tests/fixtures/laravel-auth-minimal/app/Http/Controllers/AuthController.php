<?php

namespace App\Http\Controllers;

class AuthController
{
    public function showLogin()
    {
        return view('auth.login');
    }

    public function login()
    {
        if (!$user->is_approved && strcasecmp($user->email, ADMIN_EMAIL) !== 0) {
            throw ValidationException::withMessages([]);
        }
    }
}
