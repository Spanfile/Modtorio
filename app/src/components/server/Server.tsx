import React from 'react';
import { RouteComponentProps, Switch, Redirect, Route, useParams, useRouteMatch } from 'react-router';
import { General } from './general/General';
import { Log } from './log/Log';

export default function (props: RouteComponentProps<{ id: string }>) {
    let { id } = useParams();
    let match = useRouteMatch();

    return (
        <Switch>
            <Route path={`${match.url}/general`} render={(p) => <General id={id} {...p} />} />
            <Route path={`${match.url}/log`} render={(p) => <Log id={id} {...p} />} />
            <Route path={`${match.url}/configuration`} />
            <Redirect from={match.url} to={`${match.url}/general`} />
        </Switch>
    );
}
