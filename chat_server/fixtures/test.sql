-- insert workspaces
INSERT INTO workspaces (name, owner_id) VALUES ('acme', 0);
INSERT INTO workspaces (name, owner_id) VALUES ('foo', 0);
INSERT INTO workspaces (name, owner_id) VALUES ('bar', 0);

-- insert users
INSERT INTO users (workspace_id, workspace, fullname, email, password_hash)
VALUES (1, 'acme', 'wiki', 'charmfocus@gmail.com', '$argon2id$v=19$m=19456,t=2,p=1$CkgYJ8RZSeCdCPzf4coqVg$cBfC0J5c+a/YpQXU++XEdMSGgh/LzFMXV162YqcnEEw'),
       (1, 'acme', 'wukun', 'wukun@gmail.com', '$argon2id$v=19$m=19456,t=2,p=1$CkgYJ8RZSeCdCPzf4coqVg$cBfC0J5c+a/YpQXU++XEdMSGgh/LzFMXV162YqcnEEw'),
       (1, 'foo', 'foo', 'foo@gmail.com', '$argon2id$v=19$m=19456,t=2,p=1$CkgYJ8RZSeCdCPzf4coqVg$cBfC0J5c+a/YpQXU++XEdMSGgh/LzFMXV162YqcnEEw'),
       (1, 'bar', 'bar', 'bar@gmail.com', '$argon2id$v=19$m=19456,t=2,p=1$CkgYJ8RZSeCdCPzf4coqVg$cBfC0J5c+a/YpQXU++XEdMSGgh/LzFMXV162YqcnEEw');

-- insert chats
-- insert public/private channel
INSERT INTO chats (workspace_id, name, type, members)
VALUES (1, 'general', 'public_channel', '{1,2,3,4,5}'),
    (1, 'private', 'private_channel', '{1,2,3,4,5}'),
    (1, '', 'single', '{1,2}'),
    (1, '', 'group', '{1,3,4}');
