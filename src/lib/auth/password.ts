import { scrypt, randomBytes, timingSafeEqual } from 'node:crypto';
import { promisify } from 'node:util';

const scryptAsync = promisify(scrypt);

export async function hashPassword(password: string): Promise<string> {
  const salt = randomBytes(16).toString('hex');
  const buf = (await scryptAsync(password, salt, 64)) as Buffer;
  return `${buf.toString('hex')}.${salt}`;
}

export async function verifyPassword(stored: string, input: string): Promise<boolean> {
  const [hash, salt] = stored.split('.');
  const buf = (await scryptAsync(input, salt, 64)) as Buffer;
  return timingSafeEqual(Buffer.from(hash, 'hex'), buf);
}
