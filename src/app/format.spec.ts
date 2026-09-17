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
