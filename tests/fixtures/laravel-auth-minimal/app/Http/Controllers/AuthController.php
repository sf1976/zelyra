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
        return null;
    }
}
