# 🦴 Husky
Your friendly neighborhood websocket messenger
![Screenshot](https://user-images.githubusercontent.com/45698501/162725458-45244245-66b2-4820-922f-7c25e93d3c20.png)

## What's this?
Husky is a combination of frontend and backend services made to provide websocket message exchange with encryption (todo) and no trace on server.

## How to use?
1. Clone the repo
2. Place `server.php` on your server to run as daemon
3. Modify `preconnect.php` following the comments inside  and place on the root path of the server
4. Modify `secure.php` following the comments inside (`USER_KEYS` are `username:password` formatted) and place it in the same path as `server.php`
5. Modify `secure.rs` following the comments inside
6. Build [Rust](https://www.rust-lang.org/tools/install) client app
