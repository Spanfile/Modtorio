import React from 'react';
import '../styles/App.css';

import ModNav from './ModNav';
import { Container } from 'react-bootstrap';
import AppView from './AppView';

export default function () {
  return (
    <div className="App text-left" >
      <Container>
        <ModNav />
        <AppView />
      </Container>
    </div >
  );
}
