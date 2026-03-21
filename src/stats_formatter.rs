pub struct StatsFormatter;

impl StatsFormatter {
    pub fn new(_colored: bool) -> Self {
        Self
    }

    pub fn format_stats(&self, stats: &[(String, String, u64)]) -> String {
        if stats.is_empty() {
            return "No flows collected yet...".to_string();
        }

        let count_width = "Count".len().max(
            stats
                .iter()
                .map(|(_, _, c)| c.to_string().len())
                .max()
                .unwrap_or(0),
        );
        let src_width = "Source IP"
            .len()
            .max(stats.iter().map(|(src, _, _)| src.len()).max().unwrap_or(0));
        let dst_width = "Dest IP"
            .len()
            .max(stats.iter().map(|(_, dst, _)| dst.len()).max().unwrap_or(0));

        let mut lines = Vec::new();

        let top = format!(
            "+{}+{}+{}+",
            "-".repeat(count_width + 2),
            "-".repeat(src_width + 2),
            "-".repeat(dst_width + 2)
        );
        lines.push(top);

        let header = format!(
            "| {:>count_width$} | {:<src_width$} | {:<dst_width$} |",
            "Count", "Source IP", "Dest IP"
        );
        lines.push(header);

        let sep = format!(
            "+{}+{}+{}+",
            "-".repeat(count_width + 2),
            "-".repeat(src_width + 2),
            "-".repeat(dst_width + 2)
        );
        lines.push(sep.clone());

        for (src, dst, count) in stats {
            let row = format!(
                "| {:>count_width$} | {:<src_width$} | {:<dst_width$} |",
                count, src, dst
            );
            lines.push(row);
        }

        lines.push(sep);

        lines.join("\n")
    }
}
