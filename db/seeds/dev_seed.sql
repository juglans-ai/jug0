-- dev_seed.sql
-- Development/test seed data for jug0
-- All INSERT statements use ON CONFLICT DO NOTHING for idempotency

-- ============================================
-- 1. Organizations
-- ============================================

INSERT INTO organizations (id, name, api_key_hash)
VALUES (
    'juglans_official',
    'Juglans Official',
    -- hash of 'sk_juglans_dev_key' (bcrypt)
    '$2b$12$LJ3m4ys3Lk0TSwHiPbBAhOkz6KLamMStJfGGxMFPJOFMfNe7YqHWm'
)
ON CONFLICT (id) DO NOTHING;

-- ============================================
-- 2. Users
-- ============================================

-- Admin user
INSERT INTO users (id, org_id, name, username, email, password_hash, role)
VALUES (
    '00000000-0000-0000-0000-000000000000',
    'juglans_official',
    'System Admin',
    'admin',
    'admin@juglans.io',
    '$2b$12$LJ3m4ys3Lk0TSwHiPbBAhOkz6KLamMStJfGGxMFPJOFMfNe7YqHWm', -- admin123
    'admin'
)
ON CONFLICT (id) DO NOTHING;

-- System user (for public prompts/agents)
INSERT INTO users (id, org_id, name, username, role)
VALUES (
    '00000000-0000-0000-0000-000000000001',
    'juglans_official',
    'Juglans Official',
    'juglans',
    'system'
)
ON CONFLICT (id) DO NOTHING;

-- ============================================
-- 3. System Prompts
-- ============================================

INSERT INTO prompts (id, slug, org_id, user_id, name, content, type, is_system, is_public, tags) VALUES
    ('00000000-0000-0000-0000-000000000010', 'system-default', 'juglans_official', '00000000-0000-0000-0000-000000000001',
     'Default System Prompt',
     'You are Juglans, an intelligent AI trading assistant. Help users analyze markets, execute trades, and understand financial data.',
     'system', true, true, '["system", "default"]'),

    ('00000000-0000-0000-0000-000000000011', 'market-analyst', 'juglans_official', '00000000-0000-0000-0000-000000000001',
     'Market Analyst',
     'You are a professional market analyst. Provide detailed technical and fundamental analysis of stocks, crypto, and other assets. Use charts, indicators, and market data to support your analysis.',
     'agent', true, true, '["agent", "analysis", "markets"]'),

    ('00000000-0000-0000-0000-000000000012', 'trade-executor', 'juglans_official', '00000000-0000-0000-0000-000000000001',
     'Trade Executor',
     'You are a trade execution specialist. Help users place orders, manage positions, and optimize execution. Always confirm order details before execution and explain potential risks.',
     'agent', true, true, '["agent", "trading", "execution"]'),

    ('00000000-0000-0000-0000-000000000013', 'risk-manager', 'juglans_official', '00000000-0000-0000-0000-000000000001',
     'Risk Manager',
     'You are a risk management expert. Analyze portfolio exposure, calculate position sizes, set stop-losses, and help users manage their trading risk. Always prioritize capital preservation.',
     'agent', true, true, '["agent", "risk", "portfolio"]'),

    ('00000000-0000-0000-0000-000000000014', 'news-summarizer', 'juglans_official', '00000000-0000-0000-0000-000000000001',
     'News Summarizer',
     'You are a financial news analyst. Summarize market-moving news, earnings reports, and economic events. Highlight potential trading opportunities and risks from news flow.',
     'skill', true, true, '["skill", "news", "analysis"]'),

    ('00000000-0000-0000-0000-000000000015', 'chart-reader', 'juglans_official', '00000000-0000-0000-0000-000000000001',
     'Chart Pattern Reader',
     'You are a technical analysis expert specializing in chart patterns. Identify support/resistance levels, trend lines, and classic patterns like head-and-shoulders, flags, and wedges.',
     'skill', true, true, '["skill", "technical", "charts"]'),

    ('00000000-0000-0000-0000-000000000016', 'options-analyst', 'juglans_official', '00000000-0000-0000-0000-000000000001',
     'Options Strategy Analyst',
     'You are an options trading specialist. Analyze options chains, Greeks, implied volatility, and help construct multi-leg strategies. Explain complex options concepts in simple terms.',
     'skill', true, true, '["skill", "options", "derivatives"]')
ON CONFLICT (id) DO NOTHING;

-- ============================================
-- 4. System Agents
-- ============================================

INSERT INTO agents (id, slug, org_id, user_id, name, description, default_model, system_prompt_id, temperature, is_public) VALUES
    ('00000000-0000-0000-0000-000000000020', 'default', 'juglans_official', '00000000-0000-0000-0000-000000000001',
     'Juglans AI',
     'Your intelligent trading companion. General-purpose assistant for market analysis and trading.',
     'deepseek', '00000000-0000-0000-0000-000000000010', 0.7, true),

    ('00000000-0000-0000-0000-000000000021', 'analyst', 'juglans_official', '00000000-0000-0000-0000-000000000001',
     'Market Analyst',
     'Professional market analysis with technical and fundamental insights. Perfect for research and due diligence.',
     'gpt-4o', '00000000-0000-0000-0000-000000000011', 0.5, true),

    ('00000000-0000-0000-0000-000000000022', 'trader', 'juglans_official', '00000000-0000-0000-0000-000000000001',
     'Trade Assistant',
     'Your execution partner. Helps place orders, manage positions, and optimize trade timing.',
     'claude-3-5-sonnet', '00000000-0000-0000-0000-000000000012', 0.3, true),

    ('00000000-0000-0000-0000-000000000023', 'risk-bot', 'juglans_official', '00000000-0000-0000-0000-000000000001',
     'Risk Guardian',
     'Keeps your portfolio safe. Monitors exposure, calculates position sizes, and sets protective stops.',
     'gpt-4o-mini', '00000000-0000-0000-0000-000000000013', 0.4, true),

    ('00000000-0000-0000-0000-000000000024', 'options-pro', 'juglans_official', '00000000-0000-0000-0000-000000000001',
     'Options Pro',
     'Options trading specialist. Analyzes chains, Greeks, and helps build multi-leg strategies.',
     'claude-3-5-sonnet', '00000000-0000-0000-0000-000000000016', 0.5, true)
ON CONFLICT (id) DO NOTHING;

-- ============================================
-- 5. System Workflows
-- ============================================

INSERT INTO workflows (id, slug, org_id, user_id, name, description, endpoint_url, is_active, is_public) VALUES
    ('00000000-0000-0000-0000-000000000030', 'morning-brief', 'juglans_official', '00000000-0000-0000-0000-000000000001',
     'Morning Market Brief',
     'Automated daily market summary delivered before market open. Includes overnight moves, key levels, and news highlights.',
     '/webhooks/morning-brief', true, true),

    ('00000000-0000-0000-0000-000000000031', 'earnings-alert', 'juglans_official', '00000000-0000-0000-0000-000000000001',
     'Earnings Alert Pipeline',
     'Monitors earnings calendar and triggers analysis when companies report. Summarizes results and market reaction.',
     '/webhooks/earnings', true, true),

    ('00000000-0000-0000-0000-000000000032', 'price-alert', 'juglans_official', '00000000-0000-0000-0000-000000000001',
     'Price Alert System',
     'Configurable price alerts with multi-channel notifications. Triggers when assets cross specified thresholds.',
     '/webhooks/price-alert', true, true),

    ('00000000-0000-0000-0000-000000000033', 'portfolio-rebalance', 'juglans_official', '00000000-0000-0000-0000-000000000001',
     'Portfolio Rebalancer',
     'Automated portfolio rebalancing workflow. Analyzes drift from target allocation and generates rebalance orders.',
     '/webhooks/rebalance', false, true),

    ('00000000-0000-0000-0000-000000000034', 'sentiment-scanner', 'juglans_official', '00000000-0000-0000-0000-000000000001',
     'Sentiment Scanner',
     'Scans social media and news for sentiment shifts. Aggregates signals and alerts on significant changes.',
     '/webhooks/sentiment', true, true)
ON CONFLICT (id) DO NOTHING;
