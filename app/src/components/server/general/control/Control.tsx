import React from 'react';
import { Row, Col, Button, Dropdown, SplitButton } from 'react-bootstrap';
import { ServerControlModal } from './ServerControlModal';
import { Wizard, Step } from 'components/common/Wizard';

import { ShutdownConfirmation } from './ShutdownConfirmation';
import { WaitPlayers } from './WaitPlayers';
import { Shutdown } from './Shutdown';

export function Control(props: { id: string | undefined }) {
    const [modalShow, setModalShow] = React.useState(false);

    const showWizard = () => setModalShow(true);
    const wizardDone = (result: boolean) => {
        console.log(`wizard returned ${result}`);
        setModalShow(false);
    }

    return (
        <>
            <Row xs={2} md={4} lg={6}>
                <Col className="pr-0">
                    <Button variant="outline-success" block disabled>Start</Button>
                </Col>
                <Col className="pr-0">
                    <SplitButton
                        key="shutdown"
                        id="shutdown"
                        variant="danger"
                        title="Shut down"
                        onClick={showWizard}
                    >
                        <Dropdown.Item eventKey="kill">Kill</Dropdown.Item>
                    </SplitButton>
                </Col>
                <Col className="pr-0">
                    <Button variant="warning" block>Restart</Button>
                </Col>
            </Row>
            <hr />
            <Row xs={2} md={4} lg={6}>
                <Col className="pr-0">
                    <Button variant="primary" block>Save</Button>
                </Col>
                <Col className="pr-0">
                    <Button variant="primary" block>Lua command</Button>
                </Col>
            </Row>

            <ServerControlModal variant="danger" onHide={() => wizardDone(false)} show={modalShow} title="Shut down">
                <Wizard done={wizardDone}>
                    <Step render={(p) => <ShutdownConfirmation playerCount={1} {...p} />} />
                    <Step userControlled={false} render={(p) => <WaitPlayers timeout={2} {...p} />} />
                    <Step userControlled={false} cancelable={false} render={(p) => <Shutdown {...p} />} />
                </Wizard>
            </ServerControlModal>
        </>
    );
}
