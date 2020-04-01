import React from 'react';
import '../styles/App.css';

import ModNav from './ModNav';
import ServerView from './ServerView';
import { Container } from 'react-bootstrap';
import AppView from './AppView';

export default class App extends React.Component {
  public render(): JSX.Element {
    return (
      <div className="App" >
        <Container>
          <ModNav />
          <AppView />
        </Container>
      </div >
    );
  }
}
