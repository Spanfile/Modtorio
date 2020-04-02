import React from 'react';
import { Row, Col, Button, Dropdown, SplitButton, Modal } from 'react-bootstrap';

function ServerControlModal(props: { onHide: any, show: boolean }) {
    return (
        <Modal
            {...props}
            centered
        >
            <Modal.Header closeButton>
                <Modal.Title>Shut down server</Modal.Title>
            </Modal.Header>
            <Modal.Body>
                <p>You're about to execute a server control action: Shut down</p>
            </Modal.Body>
            <Modal.Footer>
                <Button variant="secondary" className="rounded-0" onClick={props.onHide}>No</Button>
                <Button variant="danger" className="rounded-0" onClick={props.onHide}>Yes</Button>
            </Modal.Footer>
        </Modal >
    );
}

export function Control(props: { id: string | undefined }) {
    const [modalShow, setModalShow] = React.useState(false);

    const handleClose = () => setModalShow(false);
    const handleShow = () => setModalShow(true);

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
                        onClick={handleShow}
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

            <ServerControlModal onHide={handleClose} show={modalShow} />
        </>
    );
}
