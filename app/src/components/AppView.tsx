import React from 'react';
import { Switch, Route } from 'react-router-dom';
import ServerView from './ServerView';

export default class AppView extends React.Component {
    public render(): JSX.Element {
        return (
            <Switch>
                <Route path="/servers" component={ServerView} />
                <Route path={"/settings"} />
            </Switch>
        );
    }
}