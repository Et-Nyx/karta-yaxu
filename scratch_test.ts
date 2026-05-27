import { load } from './src/routes/editor/pokedex/[slug]/+page.server.ts';

async function test() {
  try {
    const result = await load({
      params: { slug: 'mydex' },
      url: new URL('http://localhost/editor/pokedex/mydex')
    } as any);
    console.log('Load Succeeded!', result.pokedex.name, 'entries count:', result.entries.length);
  } catch (err) {
    console.error('LOAD FAILED WITH ERROR:', err);
  }
}

test();
