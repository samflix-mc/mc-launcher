import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';

import { Notifications as Service } from '../../noyau/notifications';
import { Notifications } from './notifications';

/**
 * Les deux niveaux visibles : les toasts, et le centre.
 *
 * Un seul composant les porte, parce qu'ils partagent le même vocabulaire —
 * même ton, même icône, même titre. Ces tests gardent ce partage.
 */
describe('Notifications (composant)', () => {
  let service: Service;

  function monter() {
    const fixture = TestBed.createComponent(Notifications);
    fixture.detectChanges();
    return fixture;
  }

  beforeEach(() => {
    TestBed.configureTestingModule({});
    service = TestBed.inject(Service);
  });

  it('sans rien à dire, il ne dessine aucun toast', () => {
    const fixture = monter();

    expect(fixture.nativeElement.querySelector('[data-test="toasts"]')).toBeNull();
  });

  it('un avis signalé apparaît en toast, avec son titre et son détail', () => {
    service.signaler('success', 'Pack mis à jour', '128 mods');
    const fixture = monter();

    const toast = fixture.nativeElement.querySelector('[data-test="toast"]');
    expect(toast.textContent).toContain('Pack mis à jour');
    expect(toast.textContent).toContain('128 mods');
    expect(toast.dataset.ton).toBe('success');
  });

  /**
   * Un toast de danger demande une décision : il est annoncé comme une alerte,
   * là où les autres se contentent d'un statut qu'un lecteur d'écran n'
   * interrompt pas pour lire.
   */
  it('un toast de danger est une alerte, les autres un statut', () => {
    service.signaler('danger', 'Échec');
    service.signaler('info', 'Pour information');
    const fixture = monter();

    const roles = [...fixture.nativeElement.querySelectorAll('[data-test="toast"]')].map(
      (element: Element) => element.getAttribute('role'),
    );
    expect(roles).toContain('alert');
    expect(roles).toContain('status');
  });

  it('le centre est fermé tant qu’on n’a pas cliqué la cloche', () => {
    service.signaler('info', 'Un');
    const fixture = monter();

    expect(fixture.nativeElement.querySelector('[data-test="centre"]')).toBeNull();
  });

  it('ouvert, le centre liste ce qui a été dit', () => {
    service.archiver('danger', 'Quelque chose n’a pas fonctionné', 'le détail');
    service.basculerPanneau();
    const fixture = monter();

    const lignes = fixture.nativeElement.querySelectorAll('[data-test="ligne"]');
    expect(lignes.length).toBe(1);
    expect(lignes[0].textContent).toContain('le détail');
  });

  /** Un centre vide le dit, plutôt que de montrer un cadre sans contenu. */
  it('un centre vide le dit', () => {
    service.basculerPanneau();
    const fixture = monter();

    expect(fixture.nativeElement.querySelector('[data-test="centre-vide"]').textContent).toContain(
      'Rien à signaler',
    );
  });

  it('effacer vide le centre', () => {
    service.archiver('info', 'Un');
    service.basculerPanneau();
    const fixture = monter();

    fixture.nativeElement.querySelector('[data-test="vider"]').click();
    fixture.detectChanges();

    expect(fixture.nativeElement.querySelectorAll('[data-test="ligne"]').length).toBe(0);
  });

  /** Masquer un toast le retire de l'écran, jamais du centre. */
  it('masquer un toast le laisse au centre', () => {
    service.signaler('info', 'Un');
    const fixture = monter();

    fixture.nativeElement.querySelector('[data-test="fermer-toast"]').click();
    fixture.detectChanges();

    expect(fixture.nativeElement.querySelector('[data-test="toast"]')).toBeNull();
    expect(service.journal().length).toBe(1);
  });
});
