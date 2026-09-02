import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';

import App from './App';
import './styles.css';

const container = document.getElementById('root');

// Not a defensive nicety: if index.html ever loses this node, React would fail
// deep inside the renderer with a far less obvious message.
if (container === null) {
  throw new Error('Cannot mount Cliché: #root is missing from index.html');
}

createRoot(container).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
