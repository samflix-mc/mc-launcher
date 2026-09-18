import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';

import { Server } from './server';

describe('Server', () => {
  let server: Server;

  beforeEach(() => {
    TestBed.configureTestingModule({});
    server = TestBed.inject(Server);
  });

  it('has nothing before the first probe', () => {
    expect(server.status()).toBeNull();
  });

  /**
   * Outside Tauri, `refresh` requests nothing and doesn't reject — the same
   * contract as `News.load` and `Pack.refresh`, and what keeps the page
   * usable under `ng serve`.
   */
  it('outside Tauri, requests nothing', async () => {
    await expect(server.refresh()).resolves.toBeUndefined();

    expect(server.status()).toBeNull();
  });
});
