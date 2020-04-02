import React from 'react';
import { Switch, Route, Redirect } from 'react-router-dom';
import { ServerView } from './server/ServerView';

export function AppView() {
    return (
        <Switch>
            <Route path="/servers" component={ServerView} />
            <Route path={"/settings"} />
            <Redirect exact from="/" to="servers" />
        </Switch>
    );
}