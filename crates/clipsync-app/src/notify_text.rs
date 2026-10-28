//! Texto da notificação do sistema. No Linux o `notify-rust` entrega título e corpo ao daemon como
//! veio, e os daemons que seguem a especificação do freedesktop leem marcação (`<b>`, `<a href>`).
//! O nome de um par pode ter `<`, `>` e `&`, então ali esses três são escapados. No Windows o texto
//! não é interpretado, e escapar mostraria `&amp;` na tela. Módulo puro, testável sem o Tauri.

/// `&`, `<` e `>` como entidades. O `&` vai primeiro, para não escapar o que acabou de entrar.
pub fn escape_markup(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            c => out.push(c),
        }
    }
    out
}

/// O que vai ao sistema: escapado no Linux, intacto nas outras plataformas.
pub fn for_system(text: &str) -> String {
    if cfg!(target_os = "linux") {
        escape_markup(text)
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_markup_characters_become_entities() {
        assert_eq!(escape_markup("a & b"), "a &amp; b");
        assert_eq!(escape_markup("<b>x</b>"), "&lt;b&gt;x&lt;/b&gt;");
        assert_eq!(
            escape_markup("<a href=\"http://x\">clique</a>"),
            "&lt;a href=\"http://x\"&gt;clique&lt;/a&gt;"
        );
    }

    #[test]
    fn an_existing_entity_is_escaped_once_not_twice() {
        // o & do texto vira &amp; uma vez: quem lê desfaz uma vez e vê o texto original
        assert_eq!(escape_markup("&lt;"), "&amp;lt;");
        assert_eq!(escape_markup("&&<<"), "&amp;&amp;&lt;&lt;");
    }

    #[test]
    fn plain_text_accents_and_empty_are_untouched() {
        assert_eq!(escape_markup(""), "");
        assert_eq!(
            escape_markup("notebook-do-João · ção 100%"),
            "notebook-do-João · ção 100%"
        );
    }

    #[test]
    fn the_system_text_is_escaped_only_on_linux() {
        let shown = for_system("<b>pc</b> & cia");
        if cfg!(target_os = "linux") {
            assert_eq!(shown, "&lt;b&gt;pc&lt;/b&gt; &amp; cia");
        } else {
            assert_eq!(shown, "<b>pc</b> & cia");
        }
    }
}
