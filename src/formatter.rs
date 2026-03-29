use termcolor::Color;

use crate::api::flow::Verdict;
use crate::api::flow::{Endpoint, Ip, Layer4, Layer7, Service};
use crate::api::observer::get_flows_response::ResponseTypes;
use crate::api::observer::GetFlowsResponse;

const BOX_WIDTH: usize = 80;

pub struct FlowFormatter {
    colored: bool,
}

impl FlowFormatter {
    pub fn new(colored: bool) -> Self {
        Self { colored }
    }

    fn strip_ansi_codes(text: &str) -> String {
        let mut result = String::new();
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\x1b' && chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_alphabetic() {
                        chars.next();
                    } else {
                        chars.next();
                    }
                    if chars.peek().is_none() {
                        break;
                    }
                }
            } else {
                result.push(c);
            }
        }
        result
    }

    fn visible_width(text: &str) -> usize {
        Self::strip_ansi_codes(text).len()
    }

    fn truncate_text(text: &str, max_width: usize) -> String {
        let visible = Self::visible_width(text);
        if visible <= max_width {
            text.to_string()
        } else {
            let mut result = String::new();
            let mut ansi_buffer = String::new();
            let target_width = max_width.saturating_sub(3);
            let mut current_width = 0;

            let mut chars = text.chars().peekable();
            while let Some(c) = chars.next() {
                if c == '\x1b' && chars.peek() == Some(&'[') {
                    ansi_buffer.push(c);
                    chars.next();
                    while let Some(&ch) = chars.peek() {
                        ansi_buffer.push(ch);
                        chars.next();
                        if ch.is_ascii_alphabetic() {
                            break;
                        }
                    }
                    result.push_str(&ansi_buffer);
                    ansi_buffer.clear();
                } else {
                    if current_width >= target_width {
                        result.push_str(&ansi_buffer);
                        result.push_str("...");
                        return result;
                    }
                    result.push(c);
                    current_width += 1;
                }
            }
            result.push_str(&ansi_buffer);
            result.push_str("...");
            result
        }
    }

    fn box_char_h() -> &'static str {
        "═"
    }

    fn pad_line(&self, text: &str, width: usize) -> String {
        let visible = Self::visible_width(text);
        if visible >= width {
            text.to_string()
        } else {
            format!("{}{}", text, " ".repeat(width - visible))
        }
    }

    fn draw_box(&self, content: Vec<String>) -> String {
        let mut lines = Vec::new();
        lines.push(Self::box_char_h().repeat(BOX_WIDTH));
        for line in content {
            let truncated = Self::truncate_text(&line, BOX_WIDTH);
            let padded = self.pad_line(&truncated, BOX_WIDTH);
            lines.push(padded);
        }
        lines.push(Self::box_char_h().repeat(BOX_WIDTH));
        lines.join("\n")
    }

    fn draw_section(&self, title: &str, content: Vec<String>) -> Vec<String> {
        let mut lines = Vec::new();
        // Create centered header with dashes: "─ SOURCE ─"
        let header = format!("─ {} ─", title);
        lines.push(header);
        for line in content {
            let truncated = Self::truncate_text(&line, BOX_WIDTH - 2);
            lines.push(truncated);
        }
        lines
    }

    fn color_text(&self, text: &str, color: Color) -> String {
        if !self.colored {
            return text.to_string();
        }
        let ansi_code = match color {
            Color::Black => "30",
            Color::Red => "31",
            Color::Green => "32",
            Color::Yellow => "33",
            Color::Blue => "34",
            Color::Magenta => "35",
            Color::Cyan => "36",
            Color::White => "37",
            _ => "39",
        };
        format!("\x1b[{}m{}\x1b[0m", ansi_code, text)
    }

    fn color_bg_text(&self, text: &str, bg: Color, fg: Color) -> String {
        if !self.colored {
            return text.to_string();
        }
        let bg_code = match bg {
            Color::Black => "40",
            Color::Red => "41",
            Color::Green => "42",
            Color::Yellow => "43",
            Color::Blue => "44",
            Color::Magenta => "45",
            Color::Cyan => "46",
            Color::White => "47",
            _ => "49",
        };
        let fg_code = match fg {
            Color::Black => "30",
            Color::Red => "31",
            Color::Green => "32",
            Color::Yellow => "33",
            Color::Blue => "34",
            Color::Magenta => "35",
            Color::Cyan => "36",
            Color::White => "37",
            _ => "39",
        };
        format!("\x1b[{};{}m{}\x1b[0m", bg_code, fg_code, text)
    }

    fn drop_reason_to_string(reason: i32) -> String {
        match reason {
            0 => "Unknown".to_string(),
            130 => "Invalid source MAC".to_string(),
            131 => "Invalid destination MAC".to_string(),
            132 => "Invalid source IP".to_string(),
            133 => "Policy denied".to_string(),
            134 => "Invalid packet dropped".to_string(),
            135 => "CT truncated or invalid header".to_string(),
            136 => "CT missing TCP ACK flag".to_string(),
            137 => "CT unknown L4 protocol".to_string(),
            138 => "CT cannot create entry".to_string(),
            139 => "Unsupported L3 protocol".to_string(),
            140 => "Missed tail call".to_string(),
            141 => "Error writing to packet".to_string(),
            142 => "Unknown L4 protocol".to_string(),
            143 => "Unknown ICMPv4 code".to_string(),
            144 => "Unknown ICMPv4 type".to_string(),
            145 => "Unknown ICMPv6 code".to_string(),
            146 => "Unknown ICMPv6 type".to_string(),
            181 => "Policy deny".to_string(),
            _ => format!("Drop reason: {}", reason),
        }
    }

    fn format_flow_type(flow_type: i32) -> String {
        match flow_type {
            0 => "UNKNOWN".to_string(),
            1 => "L3_L4".to_string(),
            2 => "L7".to_string(),
            3 => "SOCK".to_string(),
            _ => format!("UNKNOWN({})", flow_type),
        }
    }

    pub fn format_verdict(&self, verdict: i32, drop_reason: i32) -> String {
        let (emoji, text, bg, fg) = match verdict {
            x if x == Verdict::Dropped as i32 => ("🚨", "DROPPED", Color::Red, Color::White),
            x if x == Verdict::Forwarded as i32 => ("✅", "FORWARDED", Color::Green, Color::Black),
            _ => ("⚠️", "UNKNOWN", Color::Yellow, Color::Black),
        };
        let drop_info = if drop_reason != 0 {
            format!(" - {}", Self::drop_reason_to_string(drop_reason))
        } else {
            String::new()
        };
        let header_text = format!("{} {}{}", emoji, text, drop_info);
        self.color_bg_text(&header_text, bg, fg)
    }

    pub fn format_timestamp(&self, time: &prost_types::Timestamp) -> String {
        let seconds = time.seconds as u64;
        let nanos = time.nanos as u64;
        let datetime =
            chrono::DateTime::from_timestamp(seconds as i64, nanos as u32).unwrap_or_default();
        datetime.format("%Y-%m-%d %H:%M:%S").to_string()
    }

    pub fn format_endpoint(&self, ep: &Endpoint, service: Option<&Service>) -> Vec<String> {
        let mut lines = Vec::new();
        let max_content_width = BOX_WIDTH - 10;

        // Combine namespace and pod: "Pod: namespace/pod_name"
        let pod_display = if ep.pod_name.is_empty() {
            "Unknown".to_string()
        } else {
            if !ep.namespace.is_empty() {
                format!("{}/{}", ep.namespace, ep.pod_name)
            } else {
                ep.pod_name.clone()
            }
        };
        let truncated_pod = Self::truncate_text(&pod_display, max_content_width);
        lines.push(format!(
            "Pod: {}",
            self.color_text(&truncated_pod, Color::Cyan)
        ));

        if let Some(svc) = service {
            if !svc.name.is_empty() {
                let svc_name = Self::truncate_text(&svc.name, max_content_width);
                lines.push(format!("Service: {}", svc_name));
            }
        }

        lines
    }

    pub fn format_l4(&self, l4: &Layer4, _ip: &Ip) -> Vec<String> {
        let mut lines = Vec::new();

        if let Some(protocol) = &l4.protocol {
            match protocol {
                crate::api::flow::layer4::Protocol::Tcp(tcp) => {
                    lines.push(format!("Protocol: {}", self.color_text("TCP", Color::Blue)));
                    lines.push(format!(
                        "Ports: {} → {}",
                        self.color_text(&tcp.source_port.to_string(), Color::Yellow),
                        self.color_text(&tcp.destination_port.to_string(), Color::Yellow)
                    ));

                    let mut flags = Vec::new();
                    if let Some(flags_info) = &tcp.flags {
                        if flags_info.syn {
                            flags.push("SYN");
                        }
                        if flags_info.ack {
                            flags.push("ACK");
                        }
                        if flags_info.fin {
                            flags.push("FIN");
                        }
                        if flags_info.rst {
                            flags.push("RST");
                        }
                        if flags_info.psh {
                            flags.push("PSH");
                        }
                    }
                    if !flags.is_empty() {
                        lines.push(format!("Flags: {}", flags.join(", ")));
                    }
                }
                crate::api::flow::layer4::Protocol::Udp(udp) => {
                    lines.push(format!("Protocol: {}", self.color_text("UDP", Color::Blue)));
                    lines.push(format!(
                        "Ports: {} → {}",
                        self.color_text(&udp.source_port.to_string(), Color::Yellow),
                        self.color_text(&udp.destination_port.to_string(), Color::Yellow)
                    ));
                }
                _ => {}
            }
        }

        lines
    }

    pub fn format_http(&self, http: &crate::api::flow::Http) -> Vec<String> {
        let mut lines = Vec::new();
        let max_content_width = BOX_WIDTH - 10;

        let status = match http.code {
            200 => "200 OK",
            404 => "404 Not Found",
            500 => "500 Internal Server Error",
            _ => &http.code.to_string(),
        };

        let truncated_url = Self::truncate_text(&http.url, max_content_width);
        lines.push(format!(
            "{} {} → {}",
            self.color_text(&http.method, Color::Yellow),
            truncated_url,
            status
        ));

        if !http.headers.is_empty() {
            lines.push("Headers:".to_string());
            for header in &http.headers {
                if !header.key.is_empty() && !header.value.is_empty() {
                    let truncated_key = Self::truncate_text(&header.key, max_content_width / 2);
                    let truncated_value = Self::truncate_text(&header.value, max_content_width / 2);
                    lines.push(format!("  {}: {}", truncated_key, truncated_value));
                }
            }
        }

        lines
    }

    pub fn format_dns(&self, dns: &crate::api::flow::Dns) -> Vec<String> {
        let mut lines = Vec::new();
        let max_content_width = BOX_WIDTH - 10;

        let truncated_query = Self::truncate_text(&dns.query, max_content_width);
        lines.push(format!(
            "Query: {}",
            self.color_text(&truncated_query, Color::Cyan)
        ));

        if !dns.qtypes.is_empty() {
            let qtypes = dns.qtypes.join(", ");
            let truncated_qtypes = Self::truncate_text(&qtypes, max_content_width);
            lines.push(format!("QTypes: {}", truncated_qtypes));
        }

        if !dns.ips.is_empty() {
            lines.push("Response IPs:".to_string());
            for ip in &dns.ips {
                lines.push(format!("  - {}", self.color_text(ip, Color::Magenta)));
            }
        }

        if !dns.rrtypes.is_empty() {
            let rrtypes = dns.rrtypes.join(", ");
            let truncated_rrtypes = Self::truncate_text(&rrtypes, max_content_width);
            lines.push(format!("RRTypes: {}", truncated_rrtypes));
        }

        if !dns.cnames.is_empty() {
            lines.push("CNames:".to_string());
            for cname in &dns.cnames {
                let truncated_cname = Self::truncate_text(cname, max_content_width - 4);
                lines.push(format!(
                    "  - {}",
                    self.color_text(&truncated_cname, Color::Cyan)
                ));
            }
        }

        if dns.ttl > 0 {
            lines.push(format!("TTL: {}s", dns.ttl));
        }

        if dns.rcode > 0 {
            lines.push(format!("RCODE: {}", dns.rcode));
        }

        if !dns.observation_source.is_empty() {
            let truncated_source = Self::truncate_text(&dns.observation_source, max_content_width);
            lines.push(format!("Observation Source: {}", truncated_source));
        }

        lines
    }

    pub fn format_l7(&self, l7: &Layer7) -> Vec<String> {
        let mut lines = Vec::new();

        if let Some(record) = &l7.record {
            match record {
                crate::api::flow::layer7::Record::Http(http) => {
                    lines = self.format_http(http);
                }
                crate::api::flow::layer7::Record::Dns(dns) => {
                    lines = self.format_dns(dns);
                }
                _ => {}
            }
        }

        if l7.latency_ns > 0 {
            let latency_ms = l7.latency_ns / 1_000_000;
            lines.push(format!("Latency: {}ms", latency_ms));
        }

        lines
    }

    pub fn format_flow(&self, response: &GetFlowsResponse) -> String {
        if let Some(ResponseTypes::Flow(flow)) = &response.response_types {
            let verdict = flow.verdict;
            let drop_reason = flow.drop_reason_desc;

            let mut content_lines = Vec::new();

            // 1. Time (always shown)
            if let Some(time) = &flow.time {
                content_lines.push(format!("Time: {}", self.format_timestamp(time)));
            }

            // 2. Node and Direction (always shown at verbose)
            if !flow.node_name.is_empty() {
                content_lines.push(format!("Node: {}", flow.node_name));
            }
            let direction = match flow.traffic_direction {
                1 => "Ingress",
                2 => "Egress",
                _ => "Unknown",
            };
            content_lines.push(format!("Direction: {}", direction));

            // 3. Flow Type (always shown)
            let flow_type_str = Self::format_flow_type(flow.r#type);
            content_lines.push(format!(
                "Type: {}",
                self.color_text(&flow_type_str, Color::Yellow)
            ));

            // 4. SOURCE section
            if let Some(source) = &flow.source {
                content_lines.push(String::new());
                let src_lines = self.format_endpoint(source, flow.source_service.as_ref());
                let src_section = self.draw_section("SOURCE", src_lines);
                content_lines.extend(src_section);
            }

            // 5. DESTINATION section
            if let Some(dest) = &flow.destination {
                content_lines.push(String::new());
                let dst_lines = self.format_endpoint(dest, flow.destination_service.as_ref());
                let dst_section = self.draw_section("DESTINATION", dst_lines);
                content_lines.extend(dst_section);
            }

            // 6. IP section (Source IP and Destination IP)
            if flow.ip.is_some() {
                content_lines.push(String::new());
                let ip_ref = flow.ip.as_ref().unwrap();
                let ip_lines = vec![
                    format!(
                        "Source IP: {}",
                        self.color_text(&ip_ref.source, Color::Magenta)
                    ),
                    format!(
                        "Dest IP: {}",
                        self.color_text(&ip_ref.destination, Color::Magenta)
                    ),
                ];
                let ip_section = self.draw_section("IP", ip_lines);
                content_lines.extend(ip_section);
            }

            // 7. LAYER 4 section (always shown at verbose)
            if flow.ip.is_some() && flow.l4.is_some() {
                content_lines.push(String::new());
                let ip_ref = flow.ip.as_ref().unwrap();
                if let Some(l4) = &flow.l4 {
                    let l4_lines = self.format_l4(l4, ip_ref);
                    let l4_section = self.draw_section("LAYER 4", l4_lines);
                    content_lines.extend(l4_section);
                }
            }

            // 8. LAYER 7 section (always shown at verbose)
            if flow.l7.is_some() {
                content_lines.push(String::new());
                if let Some(l7) = &flow.l7 {
                    let l7_lines = self.format_l7(l7);
                    if !l7_lines.is_empty() {
                        let l7_type = match &l7.record {
                            Some(crate::api::flow::layer7::Record::Http(_)) => "HTTP",
                            Some(crate::api::flow::layer7::Record::Dns(_)) => "DNS",
                            _ => "L7",
                        };
                        let l7_section =
                            self.draw_section(&format!("LAYER 7 ({})", l7_type), l7_lines);
                        content_lines.extend(l7_section);
                    }
                }
            }

            // 9. Add Action line at the end
            content_lines.push(String::new());
            let (emoji, text, _, _) = match verdict {
                x if x == Verdict::Dropped as i32 => ("🚨", "DROPPED", Color::Red, Color::White),
                x if x == Verdict::Forwarded as i32 => {
                    ("✅", "FORWARDED", Color::Green, Color::Black)
                }
                _ => ("⚠️", "UNKNOWN", Color::Yellow, Color::Black),
            };
            let action_text = format!("Action: {} {}", emoji, text);
            content_lines.push(action_text);

            // Draw single unified box
            self.draw_box(content_lines)
        } else {
            String::new()
        }
    }
}
