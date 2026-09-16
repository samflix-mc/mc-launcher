//! Mise en forme d'une ligne de console.

/// Mise en forme de la console : temps écoulé, niveau, message.
///
/// Le format par défaut de `tracing-subscriber` répète les champs du span
/// courant devant chaque ligne. C'est précieux dans un fichier relu plus tard,
/// illisible dans un terminal : sept lignes précédées du même
/// `commande{nom=lock manifeste=… environnement=local}` noient ce qu'on essaie
/// de lire.
///
/// Ici le message porte l'information et les champs viennent après, discrets.
/// Le fichier, lui, conserve le format complet avec les spans.
pub(super) struct ConsoleFormat {
    /// Début de l'exécution. Capturé une fois : recréer l'horloge à chaque
    /// ligne afficherait zéro partout, ce qui fut le premier essai.
    start: std::time::Instant,
}

impl ConsoleFormat {
    pub(super) fn new() -> Self {
        Self {
            start: std::time::Instant::now(),
        }
    }
}

impl<S, N> tracing_subscriber::fmt::FormatEvent<S, N> for ConsoleFormat
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
    N: for<'a> tracing_subscriber::fmt::FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &tracing_subscriber::fmt::FmtContext<'_, S, N>,
        mut writer: tracing_subscriber::fmt::format::Writer<'_>,
        event: &tracing::Event<'_>,
    ) -> std::fmt::Result {
        use tracing_subscriber::fmt::FormatFields;

        // Deux décimales suffisent à situer une étape dans une commande qui
        // dure quelques secondes ; la nanoseconde du format par défaut ne sert
        // qu'à allonger la ligne.
        write!(
            writer,
            "{:>6.2}s {:<5} ",
            self.start.elapsed().as_secs_f64(),
            event.metadata().level()
        )?;

        let level = *event.metadata().level();
        if level <= tracing::Level::INFO {
            // À partir d'`info`, le message se suffit à lui-même — c'est la
            // règle qu'on s'est donnée. Répéter les champs qu'il contient déjà
            // doublerait la ligne sans rien apprendre. Ils restent dans le
            // fichier et dans Sentry, où ils servent à filtrer.
            let mut message = MessageOnly(String::new());
            event.record(&mut message);
            write!(writer, "{}", message.0)?;
        } else {
            // En `debug` et `trace`, les champs *sont* l'information : le
            // message n'est qu'une étiquette au-dessus d'eux.
            ctx.format_fields(writer.by_ref(), event)?;
        }
        writeln!(writer)
    }
}

/// Ne retient que le champ `message` d'un événement.
struct MessageOnly(String);

impl tracing::field::Visit for MessageOnly {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.0 = format!("{value:?}");
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.0 = value.to_string();
        }
    }
}
