import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

const writeSuccess = new Rate('write_success');
const readSuccess = new Rate('read_success');
const writeLatency = new Trend('write_latency');
const readLatency = new Trend('read_latency');
const bytesWritten = new Counter('bytes_written');

const BASE_URL = __ENV.BASE_URL || 'http://127.0.0.1:5000';
const OBJECT_SIZE = parseInt(__ENV.OBJECT_SIZE || '1048576');
const WRITE_RATIO = 0.9;

export let options = {
    stages: [
        { duration: '30s', target: 10 },
        { duration: '2m', target: 50 },
        { duration: '30s', target: 0 },
    ],
    thresholds: {
        'write_success': ['rate>0.85'],
        'read_success': ['rate>0.95'],
        'write_latency': ['p(95)<1000'],
        'read_latency': ['p(95)<200'],
    },
};
