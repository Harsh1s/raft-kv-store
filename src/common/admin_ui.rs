//! Embedded admin dashboard for cluster monitoring and management.

use axum::{
    response::{Html, IntoResponse},
    routing::get,
    Router,
};

const DASHBOARD_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>minikv Admin Dashboard</title>
    <style>
        :root {
            --bg-primary: #0d1117;
            --bg-secondary: #161b22;
            --bg-tertiary: #21262d;
            --text-primary: #c9d1d9;
            --text-secondary: #8b949e;
            --accent-green: #3fb950;
            --accent-red: #f85149;
            --accent-yellow: #d29922;
            --accent-blue: #58a6ff;
            --border-color: #30363d;
        }
        
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }
        
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Helvetica, Arial, sans-serif;
            background: var(--bg-primary);
            color: var(--text-primary);
            line-height: 1.6;
        }
        
        .container {
            max-width: 1400px;
            margin: 0 auto;
            padding: 20px;
        }
        
        header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: 20px 0;
            border-bottom: 1px solid var(--border-color);
            margin-bottom: 30px;
        }
        
        .logo {
            font-size: 24px;
            font-weight: 600;
            color: var(--accent-blue);
        }
        
        .logo span {
            color: var(--text-secondary);
            font-weight: 400;
        }
        
        .version-badge {
            background: var(--bg-tertiary);
            padding: 4px 12px;
            border-radius: 20px;
            font-size: 12px;
            color: var(--text-secondary);
        }
        
        .grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
            gap: 20px;
            margin-bottom: 30px;
        }
        
        .card {
            background: var(--bg-secondary);
            border: 1px solid var(--border-color);
            border-radius: 8px;
            padding: 20px;
        }
        
        .card-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 15px;
        }
        
        .card-title {
            font-size: 14px;
            font-weight: 600;
            color: var(--text-secondary);
            text-transform: uppercase;
            letter-spacing: 0.5px;
        }
        
        .status-indicator {
            width: 10px;
            height: 10px;
            border-radius: 50%;
            display: inline-block;
        }
        
        .status-healthy { background: var(--accent-green); }
        .status-warning { background: var(--accent-yellow); }
        .status-error { background: var(--accent-red); }
        
        .metric-value {
            font-size: 36px;
            font-weight: 600;
            color: var(--text-primary);
        }
        
        .metric-label {
            font-size: 14px;
            color: var(--text-secondary);
        }
        
        .stat-row {
            display: flex;
            justify-content: space-between;
            padding: 10px 0;
            border-bottom: 1px solid var(--border-color);
        }
        
        .stat-row:last-child {
            border-bottom: none;
        }
        
        .stat-label {
            color: var(--text-secondary);
        }
        
        .stat-value {
            font-weight: 500;
        }
        
        .btn {
            display: inline-block;
            padding: 8px 16px;
            border: 1px solid var(--border-color);
            border-radius: 6px;
            background: var(--bg-tertiary);
            color: var(--text-primary);
            font-size: 14px;
            cursor: pointer;
            transition: all 0.2s;
        }
        
        .btn:hover {
            background: var(--border-color);
        }
        
        .btn-primary {
            background: var(--accent-blue);
            border-color: var(--accent-blue);
            color: white;
        }
        
        .btn-primary:hover {
            opacity: 0.9;
        }
        
        .btn-danger {
            background: var(--accent-red);
            border-color: var(--accent-red);
            color: white;
        }
        
        table {
            width: 100%;
            border-collapse: collapse;
        }
        
        th, td {
            padding: 12px;
            text-align: left;
            border-bottom: 1px solid var(--border-color);
        }
        
        th {
            font-weight: 600;
            color: var(--text-secondary);
            font-size: 12px;
            text-transform: uppercase;
        }
        
        .nav-tabs {
            display: flex;
            border-bottom: 1px solid var(--border-color);
            margin-bottom: 20px;
        }
        
        .nav-tab {
            padding: 10px 20px;
            cursor: pointer;
            border-bottom: 2px solid transparent;
            color: var(--text-secondary);
            transition: all 0.2s;
        }
        
        .nav-tab:hover {
            color: var(--text-primary);
        }
        
        .nav-tab.active {
            color: var(--accent-blue);
            border-bottom-color: var(--accent-blue);
        }
        
        .tab-content {
            display: none;
        }
        
        .tab-content.active {
            display: block;
        }
        
        .alert {
            padding: 12px 16px;
            border-radius: 6px;
            margin-bottom: 20px;
        }
        
        .alert-info {
            background: rgba(88, 166, 255, 0.1);
            border: 1px solid var(--accent-blue);
            color: var(--accent-blue);
        }
        
        .alert-success {
            background: rgba(63, 185, 80, 0.1);
            border: 1px solid var(--accent-green);
            color: var(--accent-green);
        }
        
        .alert-error {
            background: rgba(248, 81, 73, 0.1);
            border: 1px solid var(--accent-red);
            color: var(--accent-red);
        }
