<?php

Schema::create('users', function (Blueprint $table) {
    $table->id();
    $table->string('name');
    $table->string('email')->unique();
    $table->string('password');
    $table->boolean('is_approved')->default(false);
    $table->rememberToken();
    $table->timestamps();
});
