import { Lucia, TimeSpan } from 'lucia';
import { DrizzleSQLiteAdapter } from '@lucia-auth/adapter-drizzle';
import { db } from '$lib/db';
import { sessions } from '$lib/db/schema/sessions';
import { trainers } from '$lib/db/schema/trainers';

const adapter = new DrizzleSQLiteAdapter(db, sessions, trainers);

export const lucia = new Lucia(adapter, {
  sessionExpiresIn: new TimeSpan(30, 'd'),
  sessionCookie: {
    attributes: { secure: process.env.NODE_ENV === 'production' },
  },
  getUserAttributes: (attrs) => ({
    username: attrs.username,
    role:     attrs.role,
    bannedAt: attrs.bannedAt ?? null,
  }),
});

declare module 'lucia' {
  interface Register {
    Lucia: typeof lucia;
    DatabaseUserAttributes: {
      username: string;
      role: 'viewer' | 'editor';
      bannedAt: Date | null;
    };
  }
}
