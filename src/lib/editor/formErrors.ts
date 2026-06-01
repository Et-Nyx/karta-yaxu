import type { ZodError, ZodIssue } from 'zod';

export type FieldErrors = Record<string, string>;
export type FormErrorResult = { message: string; fieldErrors: FieldErrors };

export function formatZodError(
  err: ZodError,
  labels: Record<string, string> = {},
): FormErrorResult {
  const fieldErrors: FieldErrors = {};
  for (const issue of err.issues) {
    const key = String(issue.path[0] ?? '_form');
    if (fieldErrors[key]) continue;
    fieldErrors[key] = friendly(issue, labels[key] ?? humanize(key));
  }
  const n = Object.keys(fieldErrors).length;
  const message =
    n === 0 ? 'Invalid input'
    : n === 1 ? Object.values(fieldErrors)[0]
    : `${n} fields need attention`;
  return { message, fieldErrors };
}

function humanize(key: string): string {
  if (!key) return 'Field';
  return key
    .replace(/Id$/, '')
    .replace(/([A-Z])/g, ' $1')
    .replace(/^./, (c) => c.toUpperCase())
    .trim();
}

function friendly(issue: ZodIssue, label: string): string {
  const i = issue as any;
  switch (issue.code) {
    case 'too_small': {
      const min = Number(i.minimum);
      if (i.origin === 'string') {
        return min === 1 ? `${label} is required` : `${label} must be at least ${min} characters`;
      }
      if (!i.inclusive && min === 0) return `${label} is required`;
      if (i.inclusive && min === 1) return `${label} is required`;
      return i.inclusive ? `${label} must be at least ${min}` : `${label} must be greater than ${min}`;
    }
    case 'too_big': {
      const max = Number(i.maximum);
      if (i.origin === 'string') return `${label} is too long (max ${max} characters)`;
      return i.inclusive ? `${label} is too large (max ${max})` : `${label} must be less than ${max}`;
    }
    case 'invalid_format':
      return `${label} has invalid format`;
    case 'invalid_type':
      return i.input === undefined || i.input === null
        ? `${label} is required`
        : `${label} has invalid type`;
    case 'invalid_value':
      return `${label} has an invalid value`;
    case 'custom':
      return issue.message || `${label} is invalid`;
    default:
      return issue.message || `${label} is invalid`;
  }
}
