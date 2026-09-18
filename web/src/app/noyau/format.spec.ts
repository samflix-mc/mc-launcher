import * as format from './format';

describe('octets', () => {
  it('choisit une unité qui se lit', () => {
    expect(format.octets(512)).toBe('512 o');
    expect(format.octets(1_500)).toBe('2 ko');
    expect(format.octets(1_500_000)).toBe('1,5 Mo');
    expect(format.octets(830_000_000)).toBe('830 Mo');
    expect(format.octets(2_400_000_000)).toBe('2,4 Go');
  });

  it('compte en multiples de mille, comme les sources', () => {
    // Un joueur compare notre chiffre à celui affiché par Modrinth ou Mojang :
    // s'ils divergent d'un facteur 1,024 il croit à une erreur.
    expect(format.octets(1_000)).toBe('1 ko');
    expect(format.octets(1_000_000)).toBe('1,0 Mo');
  });

  it('ne décore pas les petites unités', () => {
    // « 1536,0 ko » ne se lit pas, et la décimale n'apporte rien sous le
    // mégaoctet.
    expect(format.octets(900)).toBe('900 o');
  });

  it('rend zéro pour ce qui n’est pas un nombre utilisable', () => {
    // Un total absent arrive comme zéro ; un calcul raté peut donner NaN.
    // Aucun des deux ne doit écrire « NaN o » dans la fenêtre.
    expect(format.octets(0)).toBe('0 o');
    expect(format.octets(-5)).toBe('0 o');
    expect(format.octets(Number.NaN)).toBe('0 o');
    expect(format.octets(Number.POSITIVE_INFINITY)).toBe('0 o');
  });
});

describe('debit', () => {
  it('ajoute la seconde à la taille', () => {
    expect(format.debit(8_200_000)).toBe('8,2 Mo/s');
  });
});

describe('duree', () => {
  it('donne les secondes en dessous de la minute', () => {
    expect(format.duree(0)).toBe('0 s');
    expect(format.duree(51)).toBe('51 s');
  });

  it('donne minutes et secondes au-delà', () => {
    // « 3 min » laisserait croire à une précision qu'on n'a pas ;
    // « 3 min 12 s » se lit comme un compte à rebours.
    expect(format.duree(192)).toBe('3 min 12 s');
    expect(format.duree(120)).toBe('2 min');
  });

  it('arrondit à la minute au-delà de l’heure', () => {
    expect(format.duree(3_600)).toBe('1 h');
    expect(format.duree(5_400)).toBe('1 h 30 min');
  });

  it('refuse d’inventer une durée qui n’en est pas une', () => {
    expect(format.duree(-1)).toBe('—');
    expect(format.duree(Number.NaN)).toBe('—');
  });
});

describe('pourcentage', () => {
  it('rapporte l’acquis au total', () => {
    expect(format.pourcentage(250, 1_000)).toBe(25);
  });

  it('ne dépasse jamais cent', () => {
    // Le total est un plancher quand une source ne publie pas ses tailles :
    // sans borne, la barre déborderait de sa propre largeur.
    expect(format.pourcentage(1_500, 1_000)).toBe(100);
  });

  it('rend zéro quand il n’y a rien à rapporter', () => {
    // Diviser par zéro donnerait l’infini, et une barre de largeur « Infinity% ».
    expect(format.pourcentage(0, 0)).toBe(0);
    expect(format.pourcentage(500, 0)).toBe(0);
    expect(format.pourcentage(-10, 1_000)).toBe(0);
  });
});

describe('progressionGlobale', () => {
  it('compte les étapes franchies, pas le lot courant', () => {
    // Sans cela, la barre repasse par zéro à chaque étape et « 100 % »
    // s'affiche sept fois de suite — ce qui fait douter qu'il se passe
    // quelque chose.
    expect(format.progressionGlobale(0, 0, 9)).toBe(0);
    expect(format.progressionGlobale(0, 1, 9)).toBeCloseTo(11.11, 1);
    expect(format.progressionGlobale(8, 1, 9)).toBe(100);
  });

  it('borne la fraction reçue', () => {
    // Le total est un plancher quand une source ne publie pas ses tailles :
    // la fraction peut dépasser un.
    expect(format.progressionGlobale(1, 5, 9)).toBeCloseTo(22.22, 1);
    expect(format.progressionGlobale(1, -1, 9)).toBeCloseTo(11.11, 1);
  });

  it('ne divise jamais par zéro', () => {
    expect(format.progressionGlobale(3, 0.5, 0)).toBe(0);
    expect(format.progressionGlobale(-1, 0.5, 9)).toBe(0);
  });
});

describe('teinte', () => {
  it('donne la même couleur au même compte', () => {
    expect(format.teinte('abcdef')).toBe(format.teinte('abcdef'));
  });

  it('reste dans la roue des teintes', () => {
    for (const uuid of ['a', 'cd7e6050f3ca4e4e886f44d93ac4bcc9', '']) {
      const valeur = format.teinte(uuid);
      expect(valeur).toBeGreaterThanOrEqual(0);
      expect(valeur).toBeLessThan(360);
    }
  });
});

describe('initiales', () => {
  it('prend les deux premières lettres en capitales', () => {
    expect(format.initiales('thesam1798')).toBe('TH');
  });

  it('ne rend jamais rien', () => {
    // Une pastille vide se remarque plus qu'un point d'interrogation.
    expect(format.initiales('')).toBe('?');
    expect(format.initiales('   ')).toBe('?');
  });
});
